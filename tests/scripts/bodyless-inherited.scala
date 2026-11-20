// A call whose dispatch ends at a Java class's method.
class Tick extends jlib.Counter

@main def run(): Unit = println(new Tick().bump())
