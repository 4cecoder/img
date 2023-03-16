#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <gtk/gtk.h>

#define MAX_IMAGES 100

int current_image = 0;
int total_images = 0;
GdkPixbuf* pixbuf[MAX_IMAGES];
GtkWidget* image_stack;


GdkPixbuf* load_image(const char* filename, int max_width, int max_height) {    
    GError* error = NULL;
    GdkPixbuf* pixbuf = gdk_pixbuf_new_from_file(filename, &error);
    if (error != NULL) {
        g_error_free(error);
        return NULL;
    }

    int width = gdk_pixbuf_get_width(pixbuf);
    int height = gdk_pixbuf_get_height(pixbuf);
    double scale_x = (double)max_width / width;
    double scale_y = (double)max_height / height;
    double scale = MIN(scale_x, scale_y);

    if (scale < 1.0) {
        int new_width = (int)(width * scale);
        int new_height = (int)(height * scale);
        GdkPixbuf* scaled_pixbuf = gdk_pixbuf_scale_simple(pixbuf, new_width, new_height, GDK_INTERP_BILINEAR);
        g_object_unref(pixbuf);
        return scaled_pixbuf;
    }

    return pixbuf;
}

void load_images(const char* dir_path, int max_width, int max_height) {
  DIR* dir;
  struct dirent* ent;

  dir = opendir(dir_path);
  if (dir != NULL) {
    while ((ent = readdir(dir)) != NULL) {
      if (ent->d_type == DT_REG && total_images < MAX_IMAGES) {
        const char* ext = strrchr(ent->d_name, '.');
        if (ext != NULL && (g_strcmp0(ext, ".jpg") == 0 || g_strcmp0(ext, ".jpeg") == 0 || g_strcmp0(ext, ".png") == 0)) {
          char* path = g_build_filename(dir_path, ent->d_name, NULL);
          GdkPixbuf* new_pixbuf = load_image(path, max_width, max_height);
          if (new_pixbuf != NULL) {
            pixbuf[total_images++] = new_pixbuf;
          }
          g_free(path);
        }
      }
    }
    closedir(dir);
  }
}

void update_image() {
    if (total_images > 0) {
        GtkWidget *image_widget = gtk_image_new_from_pixbuf(pixbuf[current_image]);
        gtk_stack_add_named(GTK_STACK(image_stack), image_widget, "current_image");
        gtk_widget_show(image_widget);
        gtk_stack_set_visible_child(GTK_STACK(image_stack), image_widget);
    }
}

void switch_to_image(int index) {
  current_image = index;
    
  GdkPixbuf *current_pixbuf = pixbuf[current_image];
  int width = gdk_pixbuf_get_width(current_pixbuf);
  int height = gdk_pixbuf_get_height(current_pixbuf);

  GtkWidget *toplevel = gtk_widget_get_toplevel(image);
  if (gtk_widget_is_toplevel(toplevel)) {
    gtk_window_resize(GTK_WINDOW(toplevel), width, height);
    gtk_window_set_position(GTK_WINDOW(toplevel), GTK_WIN_POS_CENTER);
  }

   update_image();
}

void on_key_press(GtkWidget* widget, GdkEventKey* event, gpointer user_data) {
    if (event->keyval == GDK_KEY_j) {
        int next_image = (current_image + 1) % total_images;
        switch_to_image(next_image);
    } else if (event->keyval == GDK_KEY_k) {
        int prev_image = (current_image - 1 + total_images) % total_images;
        switch_to_image(prev_image);
    } else if (event->keyval == GDK_KEY_q) {
        gtk_main_quit();
    }
}


int main(int argc, char** argv) {
  char* dir_path = argc == 2 ? argv[1] : ".";

  gtk_init(&argc, &argv);

  GdkDisplay *display = gdk_display_get_default();
  GdkMonitor *monitor = gdk_display_get_primary_monitor(display);
  GdkRectangle workarea;
  gdk_monitor_get_workarea(monitor, &workarea);
  int max_width = workarea.width - 20;
  int max_height = workarea.height - 20;

  load_images(dir_path, max_width, max_height);

    GtkWidget* window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "Image Viewer");
    gtk_window_set_default_size(GTK_WINDOW(window), max_width, max_height);
    gtk_container_set_border_width(GTK_CONTAINER(window), 0);
    gtk_window_set_position(GTK_WINDOW(window), GTK_WIN_POS_CENTER);
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);


    if (total_images > 0) {
        image_stack = gtk_stack_new();
        gtk_stack_set_transition_type(GTK_STACK(image_stack), GTK_STACK_TRANSITION_TYPE_CROSSFADE);
        gtk_stack_set_transition_duration(GTK_STACK(image_stack), 500);

        GtkWidget *vbox = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        gtk_box_set_center_widget(GTK_BOX(vbox), image_stack);

        gtk_container_add(GTK_CONTAINER(window), vbox);

        update_image(); // Calls 'update_image()' to set the initial image.

        g_signal_connect(window, "key-press-event", G_CALLBACK(on_key_press), NULL);
        gtk_widget_show_all(window);
        gtk_main();
    } else {
        GtkWidget *message_dialog = gtk_message_dialog_new(NULL, GTK_DIALOG_MODAL, GTK_MESSAGE_INFO, GTK_BUTTONS_OK,
                                                           "No images found in directory: %s", dir_path);
        gtk_dialog_run(GTK_DIALOG(message_dialog));
        gtk_widget_destroy(message_dialog);
    }

    for (int i = 0; i < total_images; i++) {
        g_object_unref(pixbuf[i]);
    }

    return EXIT_SUCCESS;
}
