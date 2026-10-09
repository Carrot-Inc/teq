// The Java platform layer for JavaScript, `java.net`'s sockets, beside `std/javalib/io.scala` whose
// streams they hand out (net.scala, which `--std=scala-library` keeps, holds the rest of the package).
package java.net:

  // TCP under `teq interp` (src/interp/net.rs): a server accepts one client at a time on the one
  // thread, each accept bounded by `setSoTimeout`; a socket's streams are the interpreter's, each
  // read bounded by its timeout. JavaScript has no sockets.
  @js("$fail(\"UnsupportedOperationException\", \"sockets are not available on JavaScript\")")
  def serverOpen(host: String, port: Int, backlog: Int): Int
  @js("$fail(\"UnsupportedOperationException\", \"sockets are not available on JavaScript\")")
  def serverPort(server: Int): Int
  // The client's input and output streams, its port, the local port and its address.
  @js("$fail(\"UnsupportedOperationException\", \"sockets are not available on JavaScript\")")
  def serverAccept(server: Int, timeout: Int): Array[Any]
  @js("$fail(\"UnsupportedOperationException\", \"sockets are not available on JavaScript\")")
  def serverClose(server: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"sockets are not available on JavaScript\")")
  def socketConnect(host: String, port: Int, timeout: Int): Array[Any]
  @js("$fail(\"UnsupportedOperationException\", \"sockets are not available on JavaScript\")")
  def socketTimeout(stream: Int, millis: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"sockets are not available on JavaScript\")")
  def socketShutdown(stream: Int, write: Boolean): Unit
  @js("$fail(\"UnsupportedOperationException\", \"sockets are not available on JavaScript\")")
  def resolveHost(host: String): String

  // A host's name and its numeric address.
  @javaDefined
  @jvmClass("java/net/InetAddress")
  class InetAddress private[net] (host: String, address: String):
    def getHostName: String = if host == null then address else host
    def getHostAddress: String = address
    def isLoopbackAddress: Boolean = address.startsWith("127.") || address == "::1" || address == "0:0:0:0:0:0:0:1"
    override def equals(other: Any): Boolean = other match
      case a: InetAddress => a.getHostAddress == address
      case _ => false
    override def hashCode: Int = address.hashCode
    override def toString: String = (if host == null then "" else host) + "/" + address

  @javaDefined
  @jvmClass("java/net/InetAddress")
  object InetAddress:
    def getByName(host: String): InetAddress =
      if host == null || host.isEmpty then getLoopbackAddress()
      else new InetAddress(host, resolveHost(host))
    def getLoopbackAddress(): InetAddress = new InetAddress("localhost", "127.0.0.1")

  @javaDefined
  @jvmClass("java/net/InetSocketAddress")
  class InetSocketAddress(host: String, port: Int):
    def this(port: Int) = this(null: String, port)
    def this(addr: InetAddress, port: Int) = this(if addr == null then null else addr.getHostAddress, port)
    def getHostString: String = host
    def getPort: Int = port
    override def toString: String = (if host == null then "0.0.0.0" else host) + "/<unresolved>:" + port

  @javaDefined
  @jvmClass("java/net/ServerSocket")
  class ServerSocket(port: Int, backlog: Int, bindAddr: InetAddress):
    def this(port: Int, backlog: Int) = this(port, backlog, null)
    def this(port: Int) = this(port, 50, null)
    private var id = serverOpen(if bindAddr == null then null else bindAddr.getHostAddress, port, backlog)
    // The port it was bound to, which `getLocalPort` keeps answering once it is closed, as the JDK's.
    private val boundPort = serverPort(id)
    private var timeout = 0
    def accept(): Socket =
      if id < 0 then throw new SocketException("Socket is closed")
      Socket.connected(serverAccept(id, timeout))
    def setSoTimeout(timeout: Int): Unit =
      if id < 0 then throw new SocketException("Socket is closed")
      if timeout < 0 then throw new IllegalArgumentException("timeout < 0")
      this.timeout = timeout
    def getSoTimeout(): Int =
      if id < 0 then throw new SocketException("Socket is closed")
      timeout
    def getLocalPort(): Int = boundPort
    def getInetAddress(): InetAddress = if bindAddr == null then new InetAddress(null, "0.0.0.0") else bindAddr
    def isBound(): Boolean = true
    def isClosed(): Boolean = id < 0
    def close(): Unit =
      if id >= 0 then
        serverClose(id)
        id = -1

  // A socket's streams, views of it: closing one closes the socket, as the JDK's do.
  private[net] final class SocketInput(socket: Socket, id: Int) extends java.io.NativeInputStream(id):
    override def close(): Unit = socket.close()
  private[net] final class SocketOutput(socket: Socket, id: Int) extends java.io.NativeOutputStream(id):
    override def close(): Unit = socket.close()

  // A TCP connection, the JDK's state model: unconnected (`new Socket()`, its options kept until it
  // connects), connected, then closed; its two streams, made once, are views of it; a timeout set
  // before the connection applies to it.
  @javaDefined
  @jvmClass("java/net/Socket")
  class Socket private (private var streams: Array[Any]):
    def this(host: String, port: Int) = this(socketConnect(if host == null then "localhost" else host, port, 0))
    def this(address: InetAddress, port: Int) = this(socketConnect(address.getHostAddress, port, 0))
    def this() = this(null: Array[Any])
    private var closed = false
    private var timeout = 0
    private var input: java.io.InputStream = null
    private var output: java.io.OutputStream = null
    private var inputShut = false
    private var outputShut = false
    private def open(): Unit = if closed then throw new SocketException("Socket is closed")
    private def stream(i: Int): Int =
      open()
      if streams == null then throw new SocketException("Socket is not connected")
      streams(i).asInstanceOf[Long].toInt
    def connect(endpoint: InetSocketAddress, timeout: Int): Unit =
      if endpoint == null then throw new IllegalArgumentException("connect: The address can't be null")
      if timeout < 0 then throw new IllegalArgumentException("connect: timeout can't be negative")
      open()
      if streams != null then throw new SocketException("Already connected")
      streams = socketConnect(if endpoint.getHostString == null then "localhost" else endpoint.getHostString, endpoint.getPort, timeout)
      if this.timeout > 0 then socketTimeout(stream(0), this.timeout)
    def connect(endpoint: InetSocketAddress): Unit = connect(endpoint, 0)
    def getInputStream(): java.io.InputStream =
      val id = stream(0)
      if inputShut then throw new SocketException("Socket input is shutdown")
      if input == null then input = new SocketInput(this, id)
      input
    def getOutputStream(): java.io.OutputStream =
      val id = stream(1)
      if outputShut then throw new SocketException("Socket output is shutdown")
      if output == null then output = new SocketOutput(this, id)
      output
    def setSoTimeout(timeout: Int): Unit =
      open()
      if timeout < 0 then throw new IllegalArgumentException("timeout < 0")
      this.timeout = timeout
      if streams != null then socketTimeout(stream(0), timeout)
    def getSoTimeout(): Int =
      open()
      timeout
    def setTcpNoDelay(on: Boolean): Unit = open()
    def isConnected(): Boolean = streams != null
    def isBound(): Boolean = streams != null
    def isClosed(): Boolean = closed
    def getPort(): Int = if streams == null then 0 else streams(2).asInstanceOf[Long].toInt
    def getLocalPort(): Int = if streams == null then -1 else streams(3).asInstanceOf[Long].toInt
    def getInetAddress(): InetAddress = if streams == null then null else new InetAddress(null, streams(4).asInstanceOf[String])
    def shutdownOutput(): Unit =
      val id = stream(1)
      if outputShut then throw new SocketException("Socket output is already shutdown")
      socketShutdown(id, true)
      outputShut = true
    def shutdownInput(): Unit =
      val id = stream(0)
      if inputShut then throw new SocketException("Socket input is already shutdown")
      socketShutdown(id, false)
      inputShut = true
    def isInputShutdown(): Boolean = inputShut
    def isOutputShutdown(): Boolean = outputShut
    def close(): Unit =
      if !closed then
        closed = true
        if streams != null then
          java.io.streamClose(streams(1).asInstanceOf[Long].toInt)
          java.io.streamClose(streams(0).asInstanceOf[Long].toInt)
    override def toString: String =
      if streams == null then "Socket[unconnected]"
      else "Socket[addr=/" + streams(4) + ",port=" + getPort() + ",localport=" + getLocalPort() + "]"

  @javaDefined
  @jvmClass("java/net/Socket")
  object Socket:
    private[net] def connected(streams: Array[Any]): Socket = new Socket(streams)
