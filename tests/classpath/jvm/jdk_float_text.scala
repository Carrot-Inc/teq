// jars: scala-library
// std: lean scala-library
// The reference's text of these values is the JDK's own: JDK 19 and later print the shortest
// decimal that reads back, JDK 17 (scala-cli's own where JAVA_HOME is unset and no `--jvm
// system` is given) a longer one for each (2.14748365E9, 1.9999999999999998E23, ...). A missing
// .expected is produced on the system JDK, as the JVM target's program runs.
object Main:
  def main(args: Array[String]): Unit =
    println(2147483648f)
    println(123456789f)
    println(3.0e10f)
    println(1.00000005e8f)
    println(2.0e23)
    println(1.0e23)
    println(8.41e21)
    println(4.8726570057e288)
    println(2.82879384806159e17)
