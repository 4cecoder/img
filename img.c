
int main(int argc, char *argv[]) {
    GtkWidget *window;
    GtkWidget *image;
    GdkPixbuf *pixbuf;
    GError *error = NULL;

    gtk_init(&argc, &argv);

    // Create a window
    window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "Image Viewer");
    gtk_window_set_default_size(GTK_WINDOW(window), 300, 300);
    gtk_container_set_border_width(GTK_CONTAINER(window), 10);

    // Load the image
    if (argc < 2) {
        g_print("Usage: %s <image-file>\n", argv[0]);
        return 1;
    }
    pixbuf = gdk_pixbuf_new_from_file(argv[1], &error);

    if (error != NULL) {
        g_print("Error loading image: %s\n", error->message);
        g_error_free(error);
        return 1;
    }

    // Create an image widget and display the image
    image = gtk_image_new_from_pixbuf(pixbuf);
    gtk_container_add(GTK_CONTAINER(window), image);
    gtk_widget_show_all(window);

    // Run the main loop
    gtk_main();

    // Free the image
    g_object_unref(pixbuf);

    return 0;
}
