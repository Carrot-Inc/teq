//> using platform jvm
// A socket and its streams as the JDK models them: a write reaches the peer without a flush; the
// streams are made once and are views of the socket, so that closing one closes it; shutdowns are
// states of their own; options set before the connection apply to it; a closed server keeps the
// port it was bound to. Every refusal with the JDK's exception and message.
import java.net.*

object Main:
  def show(name: String)(f: => Any): Unit =
    val r =
      try f.toString
      catch case e: Exception => e.getClass.getName + ": " + e.getMessage
    println(s"$name: $r")

  def main(args: Array[String]): Unit =
    val server = new ServerSocket(0)
    server.setSoTimeout(5000)
    val client = new Socket("127.0.0.1", server.getLocalPort())
    val peer = server.accept()
    peer.setSoTimeout(5000)
    client.getOutputStream().write(65)
    show("written without a flush")(peer.getInputStream().read())
    show("the same stream again")(client.getInputStream() eq client.getInputStream())
    client.shutdownOutput()
    show("output shut")(client.isOutputShutdown())
    show("the peer reads the end")(peer.getInputStream().read())
    show("output after its shutdown")(client.getOutputStream())
    show("shut again")(client.shutdownOutput())
    val in = client.getInputStream()
    peer.getOutputStream().write(66)
    show("read")(in.read())
    in.close()
    show("closing a stream closes the socket")(client.isClosed())
    show("read when closed")(in.read())
    show("stream when closed")(client.getInputStream())
    show("connected when closed")(client.isConnected())
    show("timeout when closed")(client.setSoTimeout(1))
    client.close()
    peer.close()
    val port = server.getLocalPort()
    server.close()
    show("closed server's port")(server.getLocalPort() == port)
    show("closed server accepting")(server.accept())
    val unconnected = new Socket()
    unconnected.setSoTimeout(50)
    show("timeout before connecting")(unconnected.getSoTimeout())
    show("stream before connecting")(unconnected.getInputStream())
    val second = new ServerSocket(0)
    unconnected.connect(new InetSocketAddress("127.0.0.1", second.getLocalPort()))
    val other = second.accept()
    show("the timeout at the connection")(unconnected.getInputStream().read())
    show("connect again")(unconnected.connect(new InetSocketAddress("127.0.0.1", second.getLocalPort())))
    show("a negative connect timeout")(new Socket().connect(new InetSocketAddress("127.0.0.1", 1), -1))
    unconnected.close()
    other.close()
    second.close()
