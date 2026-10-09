// jars: scala-library
// std: scala-library
// Two threads meet an enum's values at once: one makes them (`values`), held inside the second
// value's constructor, while the other reads the third first. The values are made together by
// whichever thread starts, and a read waits for them, as under scalac, where the companion makes
// them; a read that initialised the third value's class first waited for the enum, which waited
// for that class (the application's metrics counters).
import java.util.concurrent.CountDownLatch

object Gate:
  val making = new CountDownLatch(1)

enum Color(val tag: String):
  case Red extends Color("red")
  case Green extends Color({ Gate.making.countDown(); Thread.sleep(300); "green" })
  case Blue extends Color("blue")

@main def main(): Unit =
  @volatile var made = ""
  @volatile var read = ""
  val maker = new Thread(() => made = Color.values.map(_.tag).mkString(","))
  val reader = new Thread(() => { Gate.making.await(); read = Color.Blue.tag })
  maker.setDaemon(true)
  reader.setDaemon(true)
  reader.start()
  maker.start()
  maker.join(5000)
  reader.join(5000)
  println(if maker.isAlive || reader.isAlive then "deadlock" else s"$made $read")
