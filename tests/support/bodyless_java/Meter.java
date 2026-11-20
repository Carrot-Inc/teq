package jlib;

// An interface with an initializer and a default: a class implementing it initializes it (JVMS 5.5).
public interface Meter {
    int START = Signal.note();
    default int base() { return 1; }
}
