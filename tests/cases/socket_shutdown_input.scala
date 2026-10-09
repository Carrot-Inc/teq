//> using platform jvm
// `shutdownInput` as `NioSocketImpl` keeps it: the input stream the program already holds reads at
// its end from then on, the bytes its buffer held dropped and those the peer sends after, nothing
// available, whether read a byte, an array, by `skip` or through a reader; a second shutdown and a
// new input stream refused with the JDK's messages; the output side still writes. The second
// review's `SocketShutdown`.
import java.net.{ServerSocket, Socket, SocketException}

object Main:
  def main(args: Array[String]): Unit =
    val server = new ServerSocket(0)
    val client = new Socket("127.0.0.1", server.getLocalPort)
    val peer = server.accept()
    peer.getOutputStream().write(Array[Byte](65, 66, 67))
    val in = client.getInputStream()
    println(in.read())
    client.shutdownInput()
    println(in.read())
    println(in.available())
    peer.getOutputStream().write(Array[Byte](68, 69))
    println(in.read(new Array[Byte](4), 0, 4))
    println(in.skip(5))
    println(new java.io.InputStreamReader(in).read())
    println(client.isInputShutdown)
    println(try { client.getInputStream(); "a new stream" } catch case e: SocketException => e.getMessage)
    println(try { client.shutdownInput(); "shut again" } catch case e: SocketException => e.getMessage)
    client.getOutputStream().write(70)
    println(peer.getInputStream().read())
    client.close()
    peer.close()
    server.close()
