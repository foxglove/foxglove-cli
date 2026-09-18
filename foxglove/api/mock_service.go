package api

import (
	"context"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"sync"
	"time"

	"github.com/gorilla/mux"
)

type MockFoxgloveServer struct {
	mtx                  *sync.RWMutex
	Uploads              map[string][]byte // object storage
	IDTokens             map[string]string // device ID -> ID token
	BearerTokens         map[string]string // bearer token -> ID token
	registeredDevices    []DevicesResponse
	registeredSessions   []SessionResponse
	registeredProperties []CustomPropertiesResponseItem
	registeredEvents     []EventResponseItem
	registeredEventTypes []EventTypeResponse
	tokenRequests        int
	port                 int
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

func (s *MockFoxgloveServer) topics(w http.ResponseWriter, r *http.Request) {
	_ = json.NewEncoder(w).Encode([]TopicsResponse{{
		Encoding:       "cdr",
		SchemaEncoding: "ros2msg",
		SchemaName:     "sensor_msgs/msg/Image",
		Topic:          "/camera/image",
		Version:        "1",
	}})
}

func (s *MockFoxgloveServer) lookupDevice(id, name string) *DevicesResponse {
	for _, device := range s.registeredDevices {
		if device.ID == id || device.Name == name {
			return &device
		}
	}
	return nil
}

func (s *MockFoxgloveServer) uploadRedirect(w http.ResponseWriter, r *http.Request) {
	req := UploadRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	device := s.lookupDevice(req.DeviceID, req.DeviceName)
	if device == nil {
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
		Link: fmt.Sprintf("http://localhost:%d/storage/device_id=%s/%s", s.port, device.ID, req.Filename),
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

func (s *MockFoxgloveServer) createDevice(w http.ResponseWriter, r *http.Request) {
	req := CreateDeviceRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	resp := CreateDeviceResponse{
		ID:         fmt.Sprintf("dev_%s", req.Name),
		Name:       req.Name,
		ProjectID:  req.ProjectID,
		Properties: req.Properties,
	}

	err = json.NewEncoder(w).Encode(resp)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}

	s.mtx.RLock()
	defer s.mtx.RUnlock()
	s.registeredDevices = append(s.registeredDevices, DevicesResponse{
		ID:         resp.ID,
		Name:       resp.Name,
		ProjectID:  resp.ProjectID,
		Properties: resp.Properties,
	})
}

// Send response, but don't actually edit
func (s *MockFoxgloveServer) editDevice(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	projectId := r.URL.Query().Get("projectId")
	req := EditDeviceRequestBody{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	s.mtx.RLock()
	defer s.mtx.RUnlock()

	found := false
	for _, dev := range s.registeredDevices {
		if dev.ID == id && dev.ProjectID == projectId {
			found = true
		}
	}
	if !found {
		w.WriteHeader(http.StatusNotFound)
		return
	}

	resp := EditDeviceResponse{
		ID:         fmt.Sprintf("dev_%s", req.Name),
		Name:       req.Name,
		Properties: req.Properties,
	}

	err = json.NewEncoder(w).Encode(resp)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) devices(w http.ResponseWriter, r *http.Request) {
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	err := json.NewEncoder(w).Encode(s.registeredDevices)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) sessionsList(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	filterDeviceID := q.Get("deviceId")
	filterDeviceName := q.Get("deviceName")

	s.mtx.RLock()
	defer s.mtx.RUnlock()
	out := s.registeredSessions
	if filterDeviceID != "" || filterDeviceName != "" {
		filtered := make([]SessionResponse, 0, len(out))
		for _, sess := range out {
			if filterDeviceID != "" && (sess.Device == nil || sess.Device.ID != filterDeviceID) {
				continue
			}
			if filterDeviceName != "" && (sess.Device == nil || sess.Device.Name != filterDeviceName) {
				continue
			}
			filtered = append(filtered, sess)
		}
		out = filtered
	}
	err := json.NewEncoder(w).Encode(out)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) getSession(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	for _, sess := range s.registeredSessions {
		if sess.ID == id || sess.Key == id {
			_ = json.NewEncoder(w).Encode(sess)
			return
		}
	}
	w.WriteHeader(http.StatusNotFound)
}

func (s *MockFoxgloveServer) createSession(w http.ResponseWriter, r *http.Request) {
	req := CreateSessionRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	key, _ := randomString(8)
	resp := CreateSessionResponse{
		ID:        "sess_" + key,
		Name:      req.Name,
		Key:       key,
		ProjectID: req.ProjectID,
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}
	sess := SessionResponse{
		ID:         resp.ID,
		Name:       resp.Name,
		Key:        resp.Key,
		ProjectID:  resp.ProjectID,
		CreatedAt:  resp.CreatedAt,
		UpdatedAt:  resp.UpdatedAt,
		Recordings: []SessionRecordingSummary{},
	}
	if req.DeviceID != "" {
		sess.Device = &DeviceSummary{ID: req.DeviceID, Name: req.DeviceID}
	}
	s.mtx.Lock()
	s.registeredSessions = append(s.registeredSessions, sess)
	s.mtx.Unlock()
	_ = json.NewEncoder(w).Encode(resp)
}

func (s *MockFoxgloveServer) patchSession(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	req := PatchSessionRecordingsRequest{}
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	s.mtx.Lock()
	defer s.mtx.Unlock()
	for i := range s.registeredSessions {
		if s.registeredSessions[i].ID == id || s.registeredSessions[i].Key == id {
			recs := s.registeredSessions[i].Recordings
			if recs == nil {
				recs = []SessionRecordingSummary{}
			}
			for _, addID := range req.AddRecordingIDs {
				recs = append(recs, SessionRecordingSummary{ID: addID})
			}
			if len(req.RemoveRecordingIDs) > 0 {
				removeSet := make(map[string]bool)
				for _, rid := range req.RemoveRecordingIDs {
					removeSet[rid] = true
				}
				newRecs := make([]SessionRecordingSummary, 0, len(recs))
				for _, r := range recs {
					if !removeSet[r.ID] {
						newRecs = append(newRecs, r)
					}
				}
				recs = newRecs
			}
			s.registeredSessions[i].Recordings = recs
			s.registeredSessions[i].UpdatedAt = time.Now()
			sess := &s.registeredSessions[i]
			_ = json.NewEncoder(w).Encode(UpdateSessionResponse{
				ID:        sess.ID,
				Name:      sess.Name,
				Key:       sess.Key,
				ProjectID: sess.ProjectID,
				CreatedAt: sess.CreatedAt,
				UpdatedAt: sess.UpdatedAt,
			})
			return
		}
	}
	w.WriteHeader(http.StatusNotFound)
}

func (s *MockFoxgloveServer) eventsList(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	filterDeviceID := q.Get("deviceId")
	filterDeviceName := q.Get("deviceName")
	filterEventID := q.Get("eventId")
	filterEventTypeID := q.Get("eventTypeId")

	s.mtx.RLock()
	defer s.mtx.RUnlock()
	out := make([]EventResponseItem, 0, len(s.registeredEvents))
	for _, event := range s.registeredEvents {
		if filterDeviceID != "" && event.Device.ID != filterDeviceID {
			continue
		}
		if filterDeviceName != "" && event.Device.Name != filterDeviceName {
			continue
		}
		if filterEventID != "" && event.ID != filterEventID {
			continue
		}
		if filterEventTypeID != "" && event.EventTypeID != filterEventTypeID {
			continue
		}
		out = append(out, event)
	}
	if err := json.NewEncoder(w).Encode(out); err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) createEvent(w http.ResponseWriter, r *http.Request) {
	req := CreateEventRequest{}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if req.Start == "" || req.End == "" {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if req.EventTypeID != "" && len(req.Metadata) > 0 {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if len(req.Metadata) > 0 && len(req.Properties) > 0 {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	id, _ := randomString(8)
	device := DeviceSummary{ID: req.DeviceID, Name: req.DeviceName}
	if device.ID == "" && device.Name != "" {
		if found := s.lookupDevice("", device.Name); found != nil {
			device.ID = found.ID
			device.Name = found.Name
		}
	}
	if device.Name == "" && device.ID != "" {
		if found := s.lookupDevice(device.ID, ""); found != nil {
			device.Name = found.Name
		}
	}
	event := EventResponseItem{
		ID:          "evt_" + id,
		Device:      device,
		Start:       req.Start,
		End:         req.End,
		EventTypeID: req.EventTypeID,
		Metadata:    req.Metadata,
		Properties:  req.Properties,
		CreatedAt:   time.Now().UTC().Format(time.RFC3339),
		UpdatedAt:   time.Now().UTC().Format(time.RFC3339),
	}
	if event.Metadata == nil {
		event.Metadata = map[string]string{}
	}
	s.mtx.Lock()
	s.registeredEvents = append(s.registeredEvents, event)
	s.mtx.Unlock()
	_ = json.NewEncoder(w).Encode(event)
}

func (s *MockFoxgloveServer) findEvent(id string) (int, *EventResponseItem) {
	for i := range s.registeredEvents {
		if s.registeredEvents[i].ID == id {
			return i, &s.registeredEvents[i]
		}
	}
	return -1, nil
}

func (s *MockFoxgloveServer) getEvent(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	_, event := s.findEvent(id)
	if event == nil {
		w.WriteHeader(http.StatusNotFound)
		return
	}
	_ = json.NewEncoder(w).Encode(event)
}

func (s *MockFoxgloveServer) patchEvent(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	req := UpdateEventRequest{}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	s.mtx.Lock()
	defer s.mtx.Unlock()
	idx, event := s.findEvent(id)
	if event == nil {
		w.WriteHeader(http.StatusNotFound)
		return
	}
	if req.Start != "" {
		event.Start = req.Start
	}
	if req.End != "" {
		event.End = req.End
	}
	if req.Metadata != nil {
		event.Metadata = *req.Metadata
	}
	if req.EventTypeID != nil {
		event.EventTypeID = *req.EventTypeID
	}
	if req.Properties != nil {
		if event.Properties == nil {
			event.Properties = map[string]interface{}{}
		}
		for key, value := range req.Properties {
			if value == nil {
				delete(event.Properties, key)
				continue
			}
			event.Properties[key] = value
		}
	}
	event.UpdatedAt = time.Now().UTC().Format(time.RFC3339)
	s.registeredEvents[idx] = *event
	_ = json.NewEncoder(w).Encode(event)
}

func (s *MockFoxgloveServer) deleteEvent(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	s.mtx.Lock()
	defer s.mtx.Unlock()
	idx, _ := s.findEvent(id)
	if idx < 0 {
		w.WriteHeader(http.StatusNotFound)
		return
	}
	s.registeredEvents = append(s.registeredEvents[:idx], s.registeredEvents[idx+1:]...)
	_ = json.NewEncoder(w).Encode(map[string]string{"id": id})
}

func (s *MockFoxgloveServer) eventTypesList(w http.ResponseWriter, r *http.Request) {
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	eventTypes := make([]eventTypeAPIResponse, 0, len(s.registeredEventTypes))
	for _, eventType := range s.registeredEventTypes {
		eventTypes = append(eventTypes, eventTypeWireResponse(eventType))
	}
	if err := json.NewEncoder(w).Encode(eventTypes); err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) createEventType(w http.ResponseWriter, r *http.Request) {
	req := CreateEventTypeRequest{}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if req.Name == "" {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if req.CustomProperties == nil {
		req.CustomProperties = []EventTypeCustomProperty{}
	}
	id, _ := randomString(8)
	eventType := EventTypeResponse{
		ID:         "evtt_" + id,
		Name:       req.Name,
		ColorName:  req.ColorName,
		Properties: req.CustomProperties,
		CreatedAt:  time.Now().UTC().Format(time.RFC3339),
		UpdatedAt:  time.Now().UTC().Format(time.RFC3339),
	}
	s.mtx.Lock()
	s.registeredEventTypes = append(s.registeredEventTypes, eventType)
	s.mtx.Unlock()
	_ = json.NewEncoder(w).Encode(eventTypeWireResponse(eventType))
}

func (s *MockFoxgloveServer) findEventType(id string) (int, *EventTypeResponse) {
	for i := range s.registeredEventTypes {
		if s.registeredEventTypes[i].ID == id {
			return i, &s.registeredEventTypes[i]
		}
	}
	return -1, nil
}

func (s *MockFoxgloveServer) getEventType(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	_, eventType := s.findEventType(id)
	if eventType == nil {
		w.WriteHeader(http.StatusNotFound)
		return
	}
	_ = json.NewEncoder(w).Encode(eventTypeWireResponse(*eventType))
}

func (s *MockFoxgloveServer) patchEventType(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	req := UpdateEventTypeRequest{}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	s.mtx.Lock()
	defer s.mtx.Unlock()
	idx, eventType := s.findEventType(id)
	if eventType == nil {
		w.WriteHeader(http.StatusNotFound)
		return
	}
	if req.Name != "" {
		eventType.Name = req.Name
	}
	if req.ColorName != "" {
		eventType.ColorName = req.ColorName
	}
	if req.CustomProperties != nil {
		eventType.Properties = *req.CustomProperties
	}
	eventType.UpdatedAt = time.Now().UTC().Format(time.RFC3339)
	s.registeredEventTypes[idx] = *eventType
	_ = json.NewEncoder(w).Encode(eventTypeWireResponse(*eventType))
}

func (s *MockFoxgloveServer) deleteEventType(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	s.mtx.Lock()
	defer s.mtx.Unlock()
	idx, _ := s.findEventType(id)
	if idx < 0 {
		w.WriteHeader(http.StatusNotFound)
		return
	}
	s.registeredEventTypes = append(s.registeredEventTypes[:idx], s.registeredEventTypes[idx+1:]...)
	_ = json.NewEncoder(w).Encode(map[string]string{"id": id})
}

func (s *MockFoxgloveServer) deleteSession(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	s.mtx.Lock()
	defer s.mtx.Unlock()
	for i, sess := range s.registeredSessions {
		if sess.ID == id || sess.Key == id {
			s.registeredSessions = append(s.registeredSessions[:i], s.registeredSessions[i+1:]...)
			w.WriteHeader(http.StatusOK)
			return
		}
	}
	w.WriteHeader(http.StatusNotFound)
}

func (s *MockFoxgloveServer) imports(w http.ResponseWriter, r *http.Request) {
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	imports := []ImportsResponse{}
	for importID := range s.Uploads {
		imports = append(imports, ImportsResponse{
			ID: importID,
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

func (s *MockFoxgloveServer) uploadExtension(w http.ResponseWriter, r *http.Request) {
	_, err := io.ReadAll(r.Body)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
}

func (s *MockFoxgloveServer) listExtensions(w http.ResponseWriter, r *http.Request) {
	extensions := make([]ExtensionResponse, 0)
	err := json.NewEncoder(w).Encode(extensions)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
}

// Returns 200 if the `id` matches the mock extension name
func (s *MockFoxgloveServer) deleteExtension(w http.ResponseWriter, r *http.Request) {
	if mux.Vars(r)["id"] == s.ValidExtensionId() {
		return
	}
	w.WriteHeader(http.StatusNotFound)
}

func (s *MockFoxgloveServer) ValidExtensionId() string {
	return "ext_mock_extension_id"
}

func (s *MockFoxgloveServer) customProperties(w http.ResponseWriter, r *http.Request) {
	s.mtx.RLock()
	defer s.mtx.RUnlock()
	resourceType := r.URL.Query().Get("resourceType")
	out := s.registeredProperties
	if resourceType != "" {
		filtered := make([]CustomPropertiesResponseItem, 0, len(out))
		for _, prop := range out {
			if prop.ResourceType == resourceType {
				filtered = append(filtered, prop)
			}
		}
		out = filtered
	}
	err := json.NewEncoder(w).Encode(out)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) projects(w http.ResponseWriter, r *http.Request) {
	projects := []ProjectsResponse{
		{
			ID:             "prj_1234abcd",
			Name:           "My First Project",
			OrgMemberCount: 11,
			LastSeenAt:     nil,
		},
	}
	err := json.NewEncoder(w).Encode(projects)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func (s *MockFoxgloveServer) RegisteredProperties() []CustomPropertiesResponseItem {
	return s.registeredProperties
}

func (s *MockFoxgloveServer) RegisteredDevices() []DevicesResponse {
	return s.registeredDevices
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
				ProjectID: "prj_1234abcd",
				CreatedAt: time.Now(),
				UpdatedAt: time.Now(),
			},
		},
		registeredSessions: []SessionResponse{},
		registeredEvents:   []EventResponseItem{},
		registeredEventTypes: []EventTypeResponse{
			{
				ID:         "evtt_default",
				Name:       "Incident",
				ColorName:  "red",
				Properties: []EventTypeCustomProperty{},
				CreatedAt:  time.Now().UTC().Format(time.RFC3339),
				UpdatedAt:  time.Now().UTC().Format(time.RFC3339),
			},
		},
		registeredProperties: []CustomPropertiesResponseItem{
			{ID: "cprop_str", Key: "str", ResourceType: "device", Label: "", ValueType: "string"},
			{ID: "cprop_num", Key: "num", ResourceType: "device", Label: "", ValueType: "number"},
			{ID: "cprop_bool", Key: "bool", ResourceType: "device", Label: "", ValueType: "boolean"},
			{ID: "cprop_enum", Key: "enum", ResourceType: "device", Label: "", ValueType: "enum", Values: []string{"foo", "bar"}},
			{ID: "cprop_evt_str", Key: "severity", ResourceType: "event", Label: "Severity", ValueType: "string"},
			{ID: "cprop_evt_enum", Key: "status", ResourceType: "event", Label: "Status", ValueType: "enum", EnumValues: []string{"open", "closed"}},
			{ID: "cprop_evt_multi", Key: "tags", ResourceType: "event", Label: "Tags", ValueType: "multi-enum", EnumValues: []string{"a", "b", "c"}},
		},
	}
}

func (sv *MockFoxgloveServer) liveness(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func makeRoutes(sv *MockFoxgloveServer) *mux.Router {
	r := mux.NewRouter()
	r.HandleFunc("/v1/signin", sv.signIn).Methods("POST")
	r.HandleFunc("/v1/custom-properties", sv.withAuthz(sv.customProperties)).Methods("GET")
	r.HandleFunc("/v1/data/stream", sv.withAuthz(sv.stream)).Methods("POST")
	r.HandleFunc("/v1/data/topics", sv.withAuthz(sv.topics)).Methods("GET")
	r.HandleFunc("/v1/data/imports", sv.withAuthz(sv.imports)).Methods("GET")
	r.HandleFunc("/v1/data/upload", sv.withAuthz(sv.uploadRedirect)).Methods("POST")
	r.HandleFunc("/v1/auth/device-code", sv.deviceCode).Methods("POST")
	r.HandleFunc("/v1/auth/token", sv.token).Methods("POST")
	r.HandleFunc("/v1/devices", sv.withAuthz(sv.createDevice)).Methods("POST")
	r.HandleFunc("/v1/devices", sv.withAuthz(sv.devices)).Methods("GET")
	r.HandleFunc("/v1/devices/{id}", sv.withAuthz(sv.editDevice)).Methods("PATCH")
	r.HandleFunc("/v1/sessions", sv.withAuthz(sv.sessionsList)).Methods("GET")
	r.HandleFunc("/v1/sessions", sv.withAuthz(sv.createSession)).Methods("POST")
	r.HandleFunc("/v1/sessions/{id}", sv.withAuthz(sv.getSession)).Methods("GET")
	r.HandleFunc("/v1/sessions/{id}", sv.withAuthz(sv.patchSession)).Methods("PATCH")
	r.HandleFunc("/v1/sessions/{id}", sv.withAuthz(sv.deleteSession)).Methods("DELETE")
	r.HandleFunc("/v1/events", sv.withAuthz(sv.eventsList)).Methods("GET")
	r.HandleFunc("/v1/events", sv.withAuthz(sv.createEvent)).Methods("POST")
	r.HandleFunc("/v1/events/{id}", sv.withAuthz(sv.getEvent)).Methods("GET")
	r.HandleFunc("/v1/events/{id}", sv.withAuthz(sv.patchEvent)).Methods("PATCH")
	r.HandleFunc("/v1/events/{id}", sv.withAuthz(sv.deleteEvent)).Methods("DELETE")
	r.HandleFunc("/v1/event-types", sv.withAuthz(sv.eventTypesList)).Methods("GET")
	r.HandleFunc("/v1/event-types", sv.withAuthz(sv.createEventType)).Methods("POST")
	r.HandleFunc("/v1/event-types/{id}", sv.withAuthz(sv.getEventType)).Methods("GET")
	r.HandleFunc("/v1/event-types/{id}", sv.withAuthz(sv.patchEventType)).Methods("PATCH")
	r.HandleFunc("/v1/event-types/{id}", sv.withAuthz(sv.deleteEventType)).Methods("DELETE")
	r.HandleFunc("/v1/projects", sv.withAuthz(sv.projects)).Methods("GET")
	r.HandleFunc("/v1/extension-upload", sv.withAuthz(sv.uploadExtension)).Methods("POST")
	r.HandleFunc("/v1/extensions", sv.withAuthz(sv.listExtensions)).Methods("GET")
	r.HandleFunc("/v1/extensions/{id}", sv.withAuthz(sv.deleteExtension)).Methods("DELETE")
	r.HandleFunc("/storage/{key:.*}", sv.upload).Methods("PUT")
	r.HandleFunc("/storage/{key:.*}", sv.getStream).Methods("GET")
	r.HandleFunc("/liveness", sv.liveness).Methods("GET")
	return r
}

func randomPort() int {
	l, err := net.Listen("tcp", ":0")
	if err != nil {
		panic(fmt.Sprintf("failed to find an available port: %v", err))
	}
	defer func() { _ = l.Close() }()
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

// provides a no-op implementation of `openBrowser`
type MockAuthDelegate struct{}

func (del *MockAuthDelegate) openBrowser(url string) (*exec.Cmd, error) {
	return &exec.Cmd{
		Process: &os.Process{},
	}, nil
}
