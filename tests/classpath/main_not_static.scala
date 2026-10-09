// expect: method m cannot be a main method since it cannot be accessed statically
// A `@main` method of a class has no static path to it, as under scalac.
class K:
  @main def m(): Unit = println("m")
