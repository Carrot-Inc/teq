// The JDK's Point(int, int) stores its arguments.
class Spot extends java.awt.Point(1, 2)

@main def run(): Unit = println(new Spot().x)
