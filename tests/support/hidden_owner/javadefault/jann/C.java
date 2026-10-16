package jann;

// A Java interface's default clone, annotated: a class mixing it in has a forwarder of it that carries the
// annotation, as scalac's does.
public interface C {
  @Tag("clone")
  default Object clone() throws CloneNotSupportedException, java.io.IOException { return "ok"; }
}
