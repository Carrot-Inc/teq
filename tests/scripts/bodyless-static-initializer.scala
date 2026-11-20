// Registry has a static initializer.
@main def run(): Unit = println(new jlib.Registry() != null)
