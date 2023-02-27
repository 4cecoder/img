#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <gtk/gtk.h>

#define MAX_IMAGES 100

int current_image = 0;
int total_images = 0;
GdkPixbuf* pixbuf[MAX_IMAGES];
GtkWidget* image;

GdkPixbuf* load_image(const char* filename) {
  GError* error = NULL;
  GdkPixbuf* pixbuf = gdk_pixbuf_new_from_file(filename, &error);
  if (error != NULL) {
    g_print("Error loading image: %s\n", error->message);
    g_error_free(error);
    exit(EXIT_FAILURE);
  }
  return pixbuf;
}

void load_images(char** argv) {
  DIR* dir;
  struct dirent* ent;

  if ((dir = opendir(".")) != NULL) {
    while ((ent = readdir(dir)) != NULL) {
      if (ent->d_type == DT_REG) {
        if (total_images >= MAX_IMAGES) {
          break;
        }
        const char* ext = strrchr(ent->d_name, '.');
        if (ext != NULL && (strcmp(ext, ".jpg") == 0 || strcmp(ext, ".jpeg") == 0 || strcmp(ext, ".png") == 0)) {
          pixbuf[total_images] = load_image(ent->d_name);
          total_images++;
        }
      }
    }
    closedir(dir);
  } else {
    perror("");
    exit(EXIT_FAILURE);
  }
}

void update_image() {
  gtk_image_set_from_pixbuf(GTK_IMAGE(image), pixbuf[current_image]);
}

void switch_to_image(int index) {
  current_image = index;
  update_image();
}

void on_key_press(GtkWidget* widget, GdkEventKey* event, gpointer user_data) {
  if (event->keyval == GDK_KEY_j) {
    int next_image = (current_image + 1) % total_images;
    switch_to_image(next_image);
  } else if (event->keyval == GDK_KEY_k) {
    int prev_image = (current_image - 1 + total_images) % total_images;
    switch_to_image(prev_image);
  }
}

int main(int argc, char** argv) {
  if (argc > 1) {
    g_print("Usage: %s\n", argv[0]);
    return EXIT_FAILURE;
  }

  load_images(argv);

  gtk_init(&argc, &argv);

  GtkWidget* window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
  gtk_window_set_title(GTK_WINDOW(window), "Image Viewer");
  gtk_window_set_default_size(GTK_WINDOW(window), 300, 300);
  gtk_container_set_border_width(GTK_CONTAINER(window), 10);
  gtk_window_set_position(GTK_WINDOW(window), GTK_WIN_POS_CENTER);
  g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);

  image = gtk_image_new_from_pixbuf(pixbuf[current_image]);
  gtk_container_add(GTK_CONTAINER(window), image);

  g_signal_connect(window, "key-press-event", G_CALLBACK(on_key_press), NULL);

  gtk_widget_show_all(window);

  gtk_main();

  for (int i = 0; i < total_images; i++) {
    g_object_unref(pixbuf[i]);
  }

  return EXIT_SUCCESS;
}
