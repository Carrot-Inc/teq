//> using platform jvm
// TCP against the JDK, one thread: a server on a free port of the loopback address; an accept that
// times out; a client connected before the accept, which the system queues; a request line read
// and answered, an exchange of bytes both ways, the end of a stream seen after `shutdownOutput`;
// a read that times out; a refused connection and a closed server.
import java.io.{BufferedReader, InputStreamReader}
import java.net.*

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def main(args: Array[String]): Unit =
    val server = new ServerSocket(0, 50, InetAddress.getLoopbackAddress())
    val port = server.getLocalPort()
    show("port")(port > 0)
    show("bound")(server.getInetAddress)
    server.setSoTimeout(100)
    show("timeout")(server.getSoTimeout())
    show("accept with nobody")(server.accept())
    val client = new Socket("127.0.0.1", port)
    show("connected")(client.isConnected())
    val peer = server.accept()
    show("peer")(peer.getLocalPort() == port && peer.getPort() == client.getLocalPort() && client.getPort() == port)
    show("peer address")(peer.getInetAddress)
    val out = client.getOutputStream
    out.write("GET /a/path HTTP/1.1\r\nHost: localhost\r\n\r\n".getBytes("UTF-8"))
    out.flush()
    val in = new BufferedReader(new InputStreamReader(peer.getInputStream, "UTF-8"))
    show("request line")(in.readLine())
    show("header")(in.readLine())
    show("blank")(in.readLine().isEmpty)
    val body = "héllo".getBytes("UTF-8")
    val reply = peer.getOutputStream
    reply.write(("HTTP/1.1 200 OK\r\nContent-Length: " + body.length + "\r\n\r\n").getBytes("UTF-8"))
    reply.write(body)
    reply.flush()
    peer.shutdownOutput()
    show("response")(new String(client.getInputStream.readAllBytes(), "UTF-8").replace("\r\n", "|"))
    client.setSoTimeout(100)
    show("read timeout")({ val c2 = new Socket("127.0.0.1", port); c2.setSoTimeout(100); val p2 = server.accept(); try c2.getInputStream.read() finally { p2.close(); c2.close() } })
    client.close()
    show("closed")(client.isClosed())
    show("closed stream")(client.getInputStream)
    peer.close()
    server.close()
    show("server closed")(server.isClosed())
    show("accept closed")(server.accept())
    show("refused")(new Socket("127.0.0.1", port))
    show("unknown host")(try InetAddress.getByName("no-such-host.invalid") catch case e: UnknownHostException => e.getClass.getName)
    show("loopback")(InetAddress.getLoopbackAddress())
