GOCMD = go
GOBUILD = $(GOCMD) build
GOCLEAN = $(GOCMD) clean
TARGET = img
PREFIX = /usr/local
BINDIR = $(PREFIX)/bin

all: $(TARGET)

$(TARGET): main.go
	$(GOBUILD) -o $(TARGET) main.go

clean:
	$(GOCLEAN)

install: all
	mkdir -p $(BINDIR)
	install -m 755 $(TARGET) $(BINDIR)
