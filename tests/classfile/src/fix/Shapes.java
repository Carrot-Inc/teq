package fix;

import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;

@Retention(RetentionPolicy.RUNTIME)
@interface Marker { String value() default ""; int level() default 0; }

@Marker(value = "shapes", level = 2)
public sealed interface Shapes permits Circle, Square {
    double area();
    default String describe() { return getClass().getSimpleName() + " " + area(); }
    static Shapes unit() { return new Square(1); }
}

record Circle(double radius) implements Shapes {
    public double area() { return Math.PI * radius * radius; }
}

record Square(double side) implements Shapes {
    public double area() { return side * side; }
}

record Pair<A, B>(A first, B second) {
    static <X> Pair<X, X> twin(X x) { return new Pair<>(x, x); }
}

enum Color {
    RED(0xff0000), GREEN(0x00ff00), CUSTOM(0) {
        @Override public String label() { return "custom"; }
    };
    public final int rgb;
    Color(int rgb) { this.rgb = rgb; }
    public String label() { return name().toLowerCase(); }
}

@Deprecated
@FunctionalInterface
interface Legacy<T> extends java.io.Serializable {
    T get() throws Exception;
    @Deprecated default void reset() {}
    static final int VERSION = 1;
}
