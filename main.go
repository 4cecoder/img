package main

import (
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"strings"

	"github.com/gotk3/gotk3/gdk"
	"github.com/gotk3/gotk3/gtk"
)

var currentImage int = 0
var imagePaths []string

func loadImagePaths(directory string) {
	files, err := ioutil.ReadDir(directory)
	if err != nil {
		fmt.Println("Error: Unable to open directory.")
		return
	}

	for _, file := range files {
		if !file.IsDir() {
			filePath := filepath.Join(directory, file.Name())
			if strings.HasSuffix(strings.ToLower(filePath), ".png") || strings.HasSuffix(strings.ToLower(filePath), ".jpg") || strings.HasSuffix(strings.ToLower(filePath), ".jpeg") {
				imagePaths = append(imagePaths, filePath)
			}
		}
	}
}

func changeImage(image *gtk.Image, window *gtk.Window) {
    pixbuf, err := gdk.PixbufNewFromFile(imagePaths[currentImage])
    if err != nil {
        return
    }

    display, err := gdk.DisplayGetDefault()
    if err != nil {
        fmt.Println("Error: Unable to get default display.")
        return
    }

    monitor, err := display.GetPrimaryMonitor()
    if err != nil {
        fmt.Println("Error: Unable to get primary monitor.")
        return
    }

    monitorGeometry := monitor.GetGeometry()
    monitorWidth := monitorGeometry.GetWidth()
    monitorHeight := monitorGeometry.GetHeight()

    width := pixbuf.GetWidth()
    height := pixbuf.GetHeight()

    if width > monitorWidth {
        ratio := float64(monitorWidth) / float64(width)
        width = monitorWidth
        height = int(float64(height) * ratio)
    }

    if height > monitorHeight {
        ratio := float64(monitorHeight) / float64(height)
        height = monitorHeight
        width = int(float64(width) * ratio)
    }

    scaledPixbuf, _ := pixbuf.ScaleSimple(width, height, gdk.INTERP_BILINEAR)
    image.SetFromPixbuf(scaledPixbuf)
    window.Resize(width, height)
}

func main() {
	gtk.Init(nil)

	if len(os.Args) < 2 {
		loadImagePaths(".")
	} else {
		loadImagePaths(os.Args[1])
	}

	if len(imagePaths) == 0 {
		fmt.Println("No images found in the specified directory.")
		return
	}

	window, _ := gtk.WindowNew(gtk.WINDOW_TOPLEVEL)
	window.SetTitle("Image Viewer")
	window.SetDecorated(false)
	window.Connect("destroy", func() {
		gtk.MainQuit()
	})

	image, _ := gtk.ImageNew()
	changeImage(image, window)

	window.Connect("key-press-event", func(win *gtk.Window, event *gdk.Event) {
		keyEvent := &gdk.EventKey{Event: event}
		switch keyEvent.KeyVal() {
		case gdk.KEY_q, gdk.KEY_Q:
			gtk.MainQuit()
		case gdk.KEY_j, gdk.KEY_J:
			currentImage = (currentImage + 1) % len(imagePaths)
			changeImage(image, window)
		case gdk.KEY_k, gdk.KEY_K:
			currentImage = (currentImage - 1 + len(imagePaths)) % len(imagePaths)
			changeImage(image, window)
		}
	})

	window.Add(image)
	window.ShowAll()
	gtk.Main()
}
