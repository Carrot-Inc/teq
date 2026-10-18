// The JDK's piped streams: a circular buffer that a writing thread fills and a reading thread empties,
// each waiting for the other (in turns of a second at most, the JDK's `wait(1000)`) over a lock and a
// condition of `java.util.concurrent.locks` (std/javalib/locks.scala), with the JDK's checks of the other
// side and its messages. Under `teq interp` the two sides are logical threads; on JavaScript, which has
// one thread, a read of an empty pipe fails.
package java.io:

  import java.util.concurrent.TimeUnit
  import java.util.concurrent.locks.ReentrantLock

  @jvmClass("java/io/PipedInputStream")
  class PipedInputStream(pipeSize: Int) extends InputStream:
    if pipeSize <= 0 then throw new IllegalArgumentException("Pipe Size <= 0")
    def this() = this(1024)
    def this(src: PipedOutputStream, pipeSize: Int) =
      this(pipeSize)
      connect(src)
    def this(src: PipedOutputStream) = this(src, 1024)

    private[io] var closedByWriter = false
    private var closedByReader = false
    private[io] var connected = false
    private var readSide: Thread = null
    private var writeSide: Thread = null
    private val buffer = new Array[Byte](pipeSize)
    // Where the next byte received goes, -1 when the buffer is empty; where the next one read is.
    private var in = -1
    private var out = 0
    private[io] val lock = new ReentrantLock()
    private[io] val changed = lock.newCondition()

    def connect(src: PipedOutputStream): Unit = src.connect(this)

    // A second's wait of the other side, as the JDK's `wait(1000)`; an interrupt ends it with
    // `InterruptedIOException`.
    private def awaitOther(): Unit =
      changed.signalAll()
      try changed.await(1000L, TimeUnit.MILLISECONDS)
      catch case _: InterruptedException => throw new InterruptedIOException()

    private def checkStateForReceive(): Unit =
      if !connected then throw new IOException("Pipe not connected")
      else if closedByWriter || closedByReader then throw new IOException("Pipe closed")
      else if readSide != null && !readSide.isAlive() then throw new IOException("Read end dead")

    private[io] def receive(b: Int): Unit =
      lock.lock()
      try
        checkStateForReceive()
        writeSide = Thread.currentThread()
        while in == out do
          checkStateForReceive()
          awaitOther()
        if in < 0 then
          in = 0
          out = 0
        buffer(in) = b.toByte
        in += 1
        if in >= buffer.length then in = 0
      finally lock.unlock()

    private[io] def receivedLast(): Unit =
      lock.lock()
      try
        closedByWriter = true
        changed.signalAll()
      finally lock.unlock()

    def read(): Int =
      lock.lock()
      try
        if !connected then throw new IOException("Pipe not connected")
        else if closedByReader then throw new IOException("Pipe closed")
        else if writeSide != null && !writeSide.isAlive() && !closedByWriter && in < 0 then throw new IOException("Write end dead")
        readSide = Thread.currentThread()
        var trials = 2
        var eof = false
        while in < 0 && !eof do
          if closedByWriter then eof = true
          else
            if writeSide != null && !writeSide.isAlive() then
              trials -= 1
              if trials < 0 then throw new IOException("Pipe broken")
            awaitOther()
        if eof then -1
        else
          val ret = buffer(out) & 0xff
          out += 1
          if out >= buffer.length then out = 0
          if in == out then in = -1
          ret
      finally lock.unlock()

    override def read(b: Array[Byte], off: Int, len: Int): Int =
      if b == null then throw new NullPointerException()
      if off < 0 || len < 0 || len > b.length - off then throw new IndexOutOfBoundsException()
      if len == 0 then return 0
      val c = read()
      if c < 0 then return -1
      lock.lock()
      try
        b(off) = c.toByte
        var n = 1
        while in >= 0 && n < len do
          b(off + n) = buffer(out)
          out += 1
          n += 1
          if out >= buffer.length then out = 0
          if in == out then in = -1
        n
      finally lock.unlock()

    override def available(): Int =
      lock.lock()
      try
        if in < 0 then 0
        else if in == out then buffer.length
        else if in > out then in - out
        else in + buffer.length - out
      finally lock.unlock()

    override def close(): Unit =
      lock.lock()
      try
        closedByReader = true
        in = -1
      finally lock.unlock()

  @jvmClass("java/io/PipedOutputStream")
  class PipedOutputStream() extends OutputStream:
    private var sink: PipedInputStream = null
    def this(snk: PipedInputStream) =
      this()
      connect(snk)

    def connect(snk: PipedInputStream): Unit =
      if snk == null then throw new NullPointerException()
      if sink != null || snk.connected then throw new IOException("Already connected")
      sink = snk
      snk.connected = true

    def write(b: Int): Unit =
      if sink == null then throw new IOException("Pipe not connected")
      sink.receive(b)

    override def write(b: Array[Byte], off: Int, len: Int): Unit =
      if sink == null then throw new IOException("Pipe not connected")
      if b == null then throw new NullPointerException()
      if off < 0 || len < 0 || len > b.length - off then throw new IndexOutOfBoundsException()
      var i = 0
      while i < len do
        sink.receive(b(off + i))
        i += 1

    override def flush(): Unit =
      if sink != null then
        sink.lock.lock()
        try sink.changed.signalAll()
        finally sink.lock.unlock()

    override def close(): Unit = if sink != null then sink.receivedLast()
