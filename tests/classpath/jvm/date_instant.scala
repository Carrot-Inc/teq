// The standard library's `java.util.Date` names `java.time.Instant`, which on the JVM is the JDK's
// though no file of the standard library defines it (JavaScript takes it from scala-java-time).
object Main:
  def main(args: Array[String]): Unit =
    println(new java.util.Date(0L).toInstant)
    println(java.util.Date.from(java.time.Instant.EPOCH).toInstant == java.time.Instant.EPOCH)
    println(java.util.Date.from(java.time.Instant.ofEpochSecond(86400)).getTime())
