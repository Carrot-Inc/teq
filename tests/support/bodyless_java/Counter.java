package jlib;

// A constructor that stores its field's default, and a method no native gives.
public class Counter {
    public int count = 0;
    public Counter() {}
    public int bump() { return ++count; }
}
