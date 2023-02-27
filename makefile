CC = gcc
CFLAGS = -Wall -Wextra `pkg-config --cflags gtk+-3.0`
LDLIBS = `pkg-config --libs gtk+-3.0`

all: img

img: img.c
	$(CC) $(CFLAGS) -o $@ $< $(LDLIBS)

clean:
	rm -f img
