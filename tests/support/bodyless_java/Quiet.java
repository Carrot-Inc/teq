package jlib;

// A default without an initializer: implementing it runs nothing; calling it runs a Java body.
public interface Quiet {
    default String hum() { return "hum"; }
}
