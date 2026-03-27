package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"math"
	"math/rand"
	"os"
	"path/filepath"
	"runtime"
	"time"
)

func mean(nums []int) float64 {
	total := 0
	for _, n := range nums {
		total += n
	}
	return float64(total) / float64(len(nums))
}

func main() {
	numbers := []int{2, 4, 6, 8, 10}
	rand.Seed(42)
	randomPick := numbers[rand.Intn(len(numbers))]

	sum := sha256.Sum256([]byte("piebash"))
	digest := hex.EncodeToString(sum[:])[:16]

	cwd, _ := os.Getwd()
	payload := map[string]interface{}{
		"today":         time.Now().Format("2006-01-02"),
		"sqrt_81":       math.Sqrt(81),
		"mean":          mean(numbers),
		"random_pick":   randomPick,
		"cwd":           cwd,
		"file":          filepath.Base(os.Args[0]),
		"go_version":    runtime.Version(),
		"os":            runtime.GOOS,
		"arch":          runtime.GOARCH,
		"sha256_prefix": digest,
	}

	fmt.Println("Go runtime test:")
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	_ = enc.Encode(payload)
}
