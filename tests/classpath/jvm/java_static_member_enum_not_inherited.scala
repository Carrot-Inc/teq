// jars: scala-library
// std: scala-library
// targets: jvm
// A static member enum of a Java class (`java.net.Proxy.Type`) is not inherited: `Type` inside a class
// below `Proxy` is the program's enum.
enum Type:
  case Near, Far

class MyProx extends java.net.Proxy(java.net.Proxy.Type.HTTP, java.net.InetSocketAddress.createUnresolved("h", 1)):
  def kind: Type = Type.Far

@main def run(): Unit =
  println(new MyProx().kind)
