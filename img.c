#include <gtk/gtk.h>
#include <gdk-pixbuf/gdk-pixbuf.h>
#include <dirent.h>
#include <string.h>

#define MAX_IMAGES 100

static int current_image = 0;
static int image_count = 0;
static gchar *image_paths[MAX_IMAGES];

static void load_images(const gchar *directory) {
  DIR *dir;
  struct dirent *entry;

  if ((dir = opendir(directory)) == NULL) {
    g_print("Error: Unable to open directory.\n");
    return;
  }

  while ((entry = readdir(dir)) != NULL && image_count < MAX_IMAGES) {
    gchar *file_path = g_strdup_printf("%s/%s", directory, entry->d_name);
    if (gdk_pixbuf_new_from_file(file_path, NULL) != NULL) {
      image_paths[image_count++] = file_path;
    } else {
      g_free(file_path);
    }
  }
  closedir(dir);
}

static void change_image(GtkImage *image_widget) {
  GdkPixbuf *pixbuf = gdk_pixbuf_new_from_file(image_paths[current_image], NULL);
  if (pixbuf) {
    int width = gdk_pixbuf_get_width(pixbuf);
    int height = gdk_pixbuf_get_height(pixbuf);
    GdkDisplay *display = gdk_display_get_default();
    GdkMonitor *monitor = gdk_display_get_primary_monitor(display);
    GdkRectangle monitor_geometry;
    gdk_monitor_get_geometry(monitor, &monitor_geometry);
    int monitor_width = monitor_geometry.width;
    int monitor_height = monitor_geometry.height;

    if (width > monitor_width) {
      double ratio = (double)monitor_width / (double)width;
      width = monitor_width;
      height = (int)(height * ratio);
    }
    if (height > monitor_height) {
      double ratio = (double)monitor_height / (double)height;
      height = monitor_height;
      width = (int)(width * ratio);
    }

    GdkPixbuf *scaled_pixbuf = gdk_pixbuf_scale_simple(pixbuf, width, height, GDK_INTERP_BILINEAR);
    gtk_window_resize(GTK_WINDOW(gtk_widget_get_toplevel(GTK_WIDGET(image_widget))), width, height);
    gtk_image_set_from_pixbuf(image_widget, scaled_pixbuf);
    g_object_unref(scaled_pixbuf);
    g_object_unref(pixbuf);
  }
}

static gboolean on_key_press(GtkWidget *widget, GdkEventKey *event, gpointer user_data) {
  GtkImage *image_widget = GTK_IMAGE(user_data);

  if (event->keyval == GDK_KEY_q || event->keyval == GDK_KEY_Q) {
    gtk_main_quit();
  } else if (event->keyval == GDK_KEY_j || event->keyval == GDK_KEY_J) {
    current_image = (current_image + 1) % image_count;
    change_image(image_widget);
  } else if (event->keyval == GDK_KEY_k || event->keyval == GDK_KEY_K) {
    current_image = (current_image - 1 + image_count) % image_count;
    change_image(image_widget);
  }
  return FALSE;
}
int main(int argc, char *argv[]) {
  GtkWidget *window;
  GtkWidget *image;

  gtk_init(&argc, &argv);

  if (argc < 2) {
    load_images(".");
  } else {
    load_images(argv[1]);
  }

  if (image_count == 0) {
    g_print("No images found in the specified directory.\n");
    return 1;
  }

  window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
  gtk_window_set_decorated(GTK_WINDOW(window), FALSE); // Remove window decorations
  gtk_window_set_title(GTK_WINDOW(window), "Image Viewer");
  g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);
  g_signal_connect(window, "key_press_event", G_CALLBACK(on_key_press), NULL);

  image = gtk_image_new();
  change_image(GTK_IMAGE(image));
  g_signal_connect(window, "key_press_event", G_CALLBACK(on_key_press), image);

  gtk_container_add(GTK_CONTAINER(window), image);
  gtk_widget_show_all(window);
  gtk_main();

  for (int i = 0; i < image_count; i++) {
    g_free(image_paths[i]);
  }

  return 0;
}
