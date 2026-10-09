package scala

// The evidence `valueOf[T]` reads a singleton's value from. Link mode takes scala-library's
// class, a value class over the value, in its place (`LINKED_LAYER` in src/main.rs).
final class ValueOf[T](val value: T)
