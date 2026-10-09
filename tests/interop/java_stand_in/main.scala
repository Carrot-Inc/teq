import java.time.{Instant, Clock}

object Main:
  def main(args: Array[String]): Unit =
    println(Instant.now)
    println(Instant.now())
    println(Instant.now.plusMillis(3).toEpochMilli)
    println(Instant.ofEpochMilli(7).toEpochMilli())
    val clock = new Clock
    println(clock.millis + clock.millis())
    println(clock.zone + clock.zone())
