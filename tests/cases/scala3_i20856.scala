//> using platform jvm
// split-watch: jvm
// Adapted from scala3 tests/run/i20856.scala (Apache-2.0, see tests/scala3/README.md); replaced: the workers'
// unchecked ends, so that a worker's exception, a worker unjoined within its deadline or a passed deadline fails main.
// scalajs: --skip

import java.io.*

class Message(content: String) extends Serializable:
  //@transient
  lazy val bomb: String =
    Thread.sleep(200)
    "BOMB: " + content
end Message

object Test:
  val failures = java.util.concurrent.ConcurrentLinkedQueue[String]()
  def serialize(obj: Message): Array[Byte] =
    val byteStream = ByteArrayOutputStream()
    val objectStream = ObjectOutputStream(byteStream)
    try
      objectStream.writeObject(obj)
      byteStream.toByteArray
    finally
      objectStream.close()
      byteStream.close()
  end serialize

  def deserialize(bytes: Array[Byte]): Message =
    val byteStream = ByteArrayInputStream(bytes)
    val objectStream = ObjectInputStream(byteStream)
    try
      objectStream.readObject().asInstanceOf[Message]
    finally
      objectStream.close()
      byteStream.close()
  end deserialize

  def main(args: Array[String]): Unit =
    val bytes =
      val msg = Message("test")

      val touch = Thread(() => {
        try
          msg.bomb // start evaluation before serialization
          ()
        catch case e: Throwable => failures.add(s"touch: $e"); ()
      })
      touch.start()

      Thread.sleep(50) // give some time for the fork to start lazy val rhs eval

      val written = serialize(msg) // serialize in the meantime so that we capture Waiting state
      touch.join(10000)
      if touch.isAlive then failures.add("touch did not finish")
      written
    end bytes

    val deserializedMsg = deserialize(bytes)

    @volatile var msg = ""
    @volatile var started = false
    val read = Thread(() => {
      started = true
      try
        msg = deserializedMsg.bomb
        ()
      catch case e: Throwable => failures.add(s"read: $e"); ()
    })
    read.start()

    Thread.sleep(1000)
    if !started then
      throw Exception("ouch, the thread has not started yet after 1s")

    if !msg.isEmpty() then
      println(s"succeeded: $msg")
    else
      read.interrupt()
      throw new AssertionError("failed to read bomb in 1s!")
    read.join(10000)
    if read.isAlive then failures.add("read did not finish")
    if !failures.isEmpty then throw new AssertionError("workers failed: " + failures)
  end main
end Test
