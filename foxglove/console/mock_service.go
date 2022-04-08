package console

import (
	"context"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/gorilla/mux"
)

type MockFoxgloveServer struct {
	mtx               *sync.RWMutex
	Uploads           map[string][]byte // object storage
	IDTokens          map[string]string // device ID -> ID token
	BearerTokens      map[string]string // bearer token -> ID token
	registeredDevices []DevicesResponse
	tokenRequests     int
	port              int
}

func randomString(n int) (string, error) {
	const alphanum = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
	var bytes = make([]byte, n)
	_, err := rand.Read(bytes)
	if err != nil {
		return "", err
	}
	for i, b := range bytes {
		bytes[i] = alphanum[b%byte(len(alphanum))]
	}
	return string(bytes), nil
}

func (s *MockFoxgloveServer) BaseURL() string {
	return fmt.Sprintf("http://localhost:%d", s.port)
}

func (s *MockFoxgloveServer) signIn(w http.ResponseWriter, r *http.Request) {
	req := SignInRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	bearerToken, err := randomString(32)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	err = json.NewEncoder(w).Encode(SignInResponse{
		BearerToken: bearerToken,
	})
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	s.mtx.Lock()
	defer s.mtx.Unlock()
	s.BearerTokens[bearerToken] = req.Token
}

func (s *MockFoxgloveServer) stream(w http.ResponseWriter, r *http.Request) {
	req := StreamRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	var path string
	for k := range s.Uploads {
		if strings.HasPrefix(k, fmt.Sprintf("device_id=%s/", req.DeviceID)) {
			path = k
			break
		}
	}
	err = json.NewEncoder(w).Encode(StreamResponse{
		Link: fmt.Sprintf("http://localhost:%d/storage/%s", s.port, path),
	})
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
}

func (s *MockFoxgloveServer) hasDevice(id string) bool {
	for _, device := range s.registeredDevices {
		if device.ID == id {
			return true
		}
	}
	return false
}

func (s *MockFoxgloveServer) uploadRedirect(w http.ResponseWriter, r *http.Request) {
	req := UploadRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if !s.hasDevice(req.DeviceID) {
		w.WriteHeader(http.StatusNotFound)
		err := json.NewEncoder(w).Encode(ErrorResponse{
			Error: "Device not registered with this organization",
		})
		if err != nil {
			log.Println(err)
		}
		return
	}
	err = json.NewEncoder(w).Encode(UploadResponse{
		Link: fmt.Sprintf("http://localhost:%d/storage/device_id=%s/%s", s.port, req.DeviceID, req.Filename),
	})
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
}

func (s *MockFoxgloveServer) upload(w http.ResponseWriter, r *http.Request) {
	bytes, err := io.ReadAll(r.Body)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	key := mux.Vars(r)["key"]
	s.Uploads[key] = bytes
}

func (s *MockFoxgloveServer) devices(w http.ResponseWriter, r *http.Request) {
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	err := json.NewEncoder(w).Encode(s.registeredDevices)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) imports(w http.ResponseWriter, r *http.Request) {
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	imports := []ImportsResponse{}
	for key := range s.Uploads {
		imports = append(imports, ImportsResponse{
			ImportID: key,
		})
	}
	err := json.NewEncoder(w).Encode(imports)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) deviceCode(w http.ResponseWriter, r *http.Request) {
	req := DeviceCodeRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	deviceCode, err := randomString(6)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	err = json.NewEncoder(w).Encode(DeviceCodeResponse{
		DeviceCode: deviceCode,
	})
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	token, err := randomString(32)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	s.mtx.Lock()
	defer s.mtx.Unlock()
	s.IDTokens[deviceCode] = token
}

func (s *MockFoxgloveServer) token(w http.ResponseWriter, r *http.Request) {
	req := TokenRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	// on the first two requests, return a 403 to simulate the poll during the browser interaction
	if s.tokenRequests < 2 {
		s.tokenRequests++
		w.WriteHeader(http.StatusForbidden)
		return
	}

	if token, ok := s.IDTokens[req.DeviceCode]; ok {
		err = json.NewEncoder(w).Encode(TokenResponse{
			IDToken: token,
		})
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			return
		}
	} else {
		w.WriteHeader(http.StatusForbidden)
	}
}

func (s *MockFoxgloveServer) getStream(w http.ResponseWriter, r *http.Request) {
	s.mtx.Lock()
	defer s.mtx.Unlock()
	key := mux.Vars(r)["key"]
	if bytes, ok := s.Uploads[key]; ok {
		_, err := w.Write(bytes)
		if err != nil {
			fmt.Println(err)
		}
	} else {
		_, err := w.Write([]byte{})
		if err != nil {
			fmt.Println(err)
		}
	}
}

func (s *MockFoxgloveServer) withAuthz(next func(http.ResponseWriter, *http.Request)) func(http.ResponseWriter, *http.Request) {
	return func(w http.ResponseWriter, r *http.Request) {
		parts := strings.Split(r.Header.Get("Authorization"), " ")
		if len(parts) != 2 {
			w.WriteHeader(http.StatusForbidden)
			return
		}
		s.mtx.RLock()
		if _, ok := s.BearerTokens[parts[1]]; !ok {
			s.mtx.RUnlock()
			w.WriteHeader(http.StatusForbidden)
			return
		}
		s.mtx.RUnlock()
		next(w, r)
	}
}

func mockServer(port int) *MockFoxgloveServer {
	return &MockFoxgloveServer{
		mtx:           &sync.RWMutex{},
		Uploads:       make(map[string][]byte),
		IDTokens:      make(map[string]string),
		BearerTokens:  make(map[string]string),
		tokenRequests: 0,
		port:          port,
		registeredDevices: []DevicesResponse{
			{
				ID:        "test-device",
				Name:      "my test device",
				CreatedAt: time.Now(),
				UpdatedAt: time.Now(),
			},
		},
	}
}

func (sv *MockFoxgloveServer) liveness(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func makeRoutes(sv *MockFoxgloveServer) *mux.Router {
	r := mux.NewRouter()
	r.HandleFunc("/v1/signin", sv.signIn).Methods("POST")
	r.HandleFunc("/v1/data/stream", sv.withAuthz(sv.stream)).Methods("POST")
	r.HandleFunc("/v1/data/imports", sv.withAuthz(sv.imports)).Methods("GET")
	r.HandleFunc("/v1/data/upload", sv.withAuthz(sv.uploadRedirect)).Methods("POST")
	r.HandleFunc("/v1/auth/device-code", sv.deviceCode).Methods("POST")
	r.HandleFunc("/v1/auth/token", sv.token).Methods("POST")
	r.HandleFunc("/v1/devices", sv.withAuthz(sv.devices)).Methods("GET")
	r.HandleFunc("/storage/{key:.*}", sv.upload).Methods("PUT")
	r.HandleFunc("/storage/{key:.*}", sv.getStream).Methods("GET")
	r.HandleFunc("/liveness", sv.liveness).Methods("GET")
	return r
}

func randomPort() int {
	l, _ := net.Listen("tcp", ":0")
	defer l.Close()
	return l.Addr().(*net.TCPAddr).Port
}

// NewMockServer returns a new mock server. Canceling the supplied context will
// terminate the server.
func NewMockServer(ctx context.Context) (*MockFoxgloveServer, error) {
	port := randomPort()
	sv := mockServer(port)
	routes := makeRoutes(sv)
	srv := &http.Server{
		Addr:    fmt.Sprintf(":%d", port),
		Handler: routes,
	}
	go func() {
		<-ctx.Done()
		err := srv.Shutdown(ctx)
		if err != nil {
			log.Printf("error shutting down mock server: %v", err)
		}
	}()

	// start the server
	go func() {
		if err := srv.ListenAndServe(); err != http.ErrServerClosed {
			log.Printf("HTTP server ListenAndServe: %v", err)
		}
	}()

	// poll liveness endpoint until server is up
	for {
		select {
		case <-ctx.Done():
			return nil, fmt.Errorf("startup timeout")
		default:
		}
		resp, err := http.Get(sv.BaseURL() + "/liveness")
		if err != nil {
			continue
		}
		if resp.StatusCode == http.StatusOK {
			break
		}
	}
	return sv, nil
}
