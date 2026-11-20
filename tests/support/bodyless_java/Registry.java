package jlib;

// A static initializer, which prints and sets a static field no native gives.
public class Registry {
    public static final java.util.List<String> NAMES = new java.util.ArrayList<>();
    public static int hits = 5;
    static { System.out.println("registry-init"); }
    public Registry() {}
}
