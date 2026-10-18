//> using dep dev.zio::zio:2.1.25
// zio 2.1.25's RingBuffer, whose Java padding class has the static initialiser the interpreter knows at zio
// 2.1.26 alone: the class of another version is refused, by its jar's manifest (tests/interp-script.sh's
// `refusal_cp`).
package zio
object Main:
  def main(args: Array[String]): Unit =
    val q = internal.RingBuffer[String](4)
    println(q.offer("value"))
    println(q.poll(null))
