package hidden;

// A public class whose methods answer the package-private classes themselves.
public class Factory {
    public static Inner make() { return new Inner(); }
    public static Base base() { return new Inner(); }
}
