package tablewriter

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
)

func computeHotdogCellWidths(headers []string, data [][]string) (int, []int) {
	cellWidths := make([]int, len(headers))
	for i, header := range headers {
		headerWidth := len(header) + 4 // pad two spaces each side
		cellWidths[i] = headerWidth
	}
	for _, row := range data {
		for i, column := range row {
			columnWidth := len(column) + 2 // pad one space per side
			if cellWidths[i] < columnWidth {
				cellWidths[i] = columnWidth
			}
		}
	}

	// size the cells so the headers can be center-spaced
	for i, header := range headers {
		if (cellWidths[i]-len(header))%2 == 1 {
			cellWidths[i] += 1
		}
	}

	tableWidth := len(headers) + 1
	for _, width := range cellWidths {
		tableWidth += width
	}

	return tableWidth, cellWidths
}

/*
printHotDog outputs a table of records formatted like this:
|          ID          |         Name         |      Created At      |      Updated At      |
|----------------------|----------------------|----------------------|----------------------|
| dev_qOo9LfqjfymSj50y | hilti-handheld       | 2023-06-01T11:37:55Z | 2023-06-01T11:37:55Z |
| dev_kmJaeAdpSyLkqORp | my-new-device        | 2023-05-26T16:40:38Z | 2023-05-26T16:40:38Z |
*/
func printHotDog(w io.Writer, headers []string, data [][]string) {
	_, cellWidths := computeHotdogCellWidths(headers, data)

	// write the headers
	fmt.Fprintf(w, "|")
	for i, header := range headers {
		padding := (cellWidths[i] - len(header)) / 2
		fmt.Fprintf(w, "%s%s%s|", strings.Repeat(" ", padding), header, strings.Repeat(" ", padding))
	}
	fmt.Fprintln(w)

	// write the separator
	fmt.Fprintf(w, "|")
	for _, width := range cellWidths {
		fmt.Fprint(w, strings.Repeat("-", width))
		fmt.Fprintf(w, "|")
	}
	fmt.Fprintln(w)

	// write the data
	for _, row := range data {
		fmt.Fprint(w, "|")
		for i, col := range row {
			fmt.Fprintf(w, " %s%s|", col, strings.Repeat(" ", cellWidths[i]-len(col)-1))
		}
		fmt.Fprintln(w)
	}
}

/*
printHamburger outputs a series of records formatted like this:

	-[ RECORD 17 ]+-----------------------------------
	ID            | dev_zZFmSJfwI4OX4HJq
	Name          | roman-gps-mcap
	Created At    | 2021-11-17T18:23:49Z
	Updated At    | 2021-11-17T18:23:49Z
*/
func printHamburger(w io.Writer, termwidth int, headers []string, data [][]string) {
	var maxHeaderWidth int
	var maxRecordWidth int

	// compute the max header width
	for _, header := range headers {
		if len(header) > maxHeaderWidth {
			maxHeaderWidth = len(header)
		}
	}

	// compute the max record width
	for _, row := range data {
		for _, col := range row {
			if len(col) > maxRecordWidth {
				maxRecordWidth = len(col)
			}
		}
	}

	// ensure there is sufficient room to accommodate the highest record header
	// required.
	longestRecordHeader := fmt.Sprintf("-[ RECORD %d ]", len(data)+1)
	if len(longestRecordHeader) > maxHeaderWidth {
		maxHeaderWidth = len(longestRecordHeader)
	}

	// rightExtent is how far to extend  the dashes after the +. If there's
	// sufficient room, set it 15 spaces farther than the max record width. If
	// this would cause a wrap, set it to fill the term width.
	dashesRightExtent := maxRecordWidth + 15
	maxAllowedExtent := termwidth - maxHeaderWidth - 1
	if dashesRightExtent > maxAllowedExtent {
		dashesRightExtent = maxAllowedExtent
	}
	rightDashes := strings.Repeat("-", dashesRightExtent)

	// print the records
	for i, row := range data {
		recordHeader := fmt.Sprintf("-[ RECORD %d ]", i+1)
		header := fmt.Sprintf(
			"%s%s+%s",
			recordHeader,
			strings.Repeat("-", maxHeaderWidth-len(recordHeader)),
			rightDashes,
		)
		fmt.Fprintln(w, header)
		for j, col := range row {
			fmt.Fprintf(w, "%-*s| %-*s", maxHeaderWidth, headers[j], dashesRightExtent-1, col)
			fmt.Fprintln(w)
		}
	}
}

func getTermWidth() int {
	cmd := exec.Command("stty", "size")
	cmd.Stdin = os.Stdin
	out, err := cmd.Output()
	if err != nil {
		return 80
	}
	var rows, cols int
	_, err = fmt.Sscanf(string(out), "%d %d", &rows, &cols)
	if err != nil {
		return 80
	}
	return cols
}

func PrintTable(w io.Writer, headers []string, data [][]string) {
	termWidth := getTermWidth()
	tableWidth, _ := computeHotdogCellWidths(headers, data)
	if termWidth < tableWidth {
		printHamburger(w, termWidth, headers, data)
		return
	}
	printHotDog(w, headers, data)
}
