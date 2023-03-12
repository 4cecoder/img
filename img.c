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
        g_error_free(error);
        return NULL;
    }
    return pixbuf;
}

void load_images(const char* dir_path) {
    DIR* dir;
    struct dirent* ent;

    if ((dir = opendir(dir_path)) != NULL) {
        while ((ent = readdir(dir)) != NULL) {
            if (ent->d_type == DT_REG && total_images < MAX_IMAGES) {
                const char* ext = strrchr(ent->d_name, '.');
                if (ext != NULL && (strcmp(ext, ".jpg") == 0 || strcmp(ext, ".jpeg") == 0 || strcmp(ext, ".png") == 0)) {
                    char* path = g_build_filename(dir_path, ent->d_name, NULL);
                    GdkPixbuf* new_pixbuf = load_image(path);
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
        gtk_image_set_from_pixbuf(GTK_IMAGE(image), pixbuf[current_image]);
    }
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
    } else if (event->keyval == GDK_KEY_q) {
        gtk_main_quit();
    }
}


int main(int argc, char** argv) {
    char* dir_path = argc == 2 ? argv[1] : ".";
    load_images(dir_path);

    gtk_init(&argc, &argv);

    GdkDisplay *display = gdk_display_get_default();
    GdkMonitor *monitor = gdk_display_get_primary_monitor(display);
    GdkRectangle workarea;
    gdk_monitor_get_workarea(monitor, &workarea);
    int max_width = workarea.width - 20;
    int max_height = workarea.height - 20;

    GtkWidget* window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "Image Viewer");
    gtk_window_set_default_size(GTK_WINDOW(window), max_width, max_height);
    gtk_container_set_border_width(GTK_CONTAINER(window), 10);
    gtk_window_set_position(GTK_WINDOW(window), GTK_WIN_POS_CENTER);
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);

    if (total_images > 0) {
        image = gtk_image_new_from_pixbuf(pixbuf[current_image]);
        gtk_container_add(GTK_CONTAINER(window), image);
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
