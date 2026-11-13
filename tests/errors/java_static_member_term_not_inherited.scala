// jars: scala-library
// teq: --target jvm
// A static member enum of a Java class by its simple name inside a subclass: not found, as scalac 3.8.4
// says (E006).
// expect: 7:19: error: not found: Type
class MyProx extends java.net.Proxy(java.net.Proxy.Type.HTTP, java.net.InetSocketAddress.createUnresolved("h", 1)):
  def http: Any = Type.HTTP

@main def run(): Unit =
  println(new MyProx().http)
