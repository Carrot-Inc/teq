// `Class.forName` of an array descriptor loads its component as the JDK's `Class.forName` does, without
// initialising it, and an absent component is a `ClassNotFoundException` (the review's `ForNameMore`, its lines the
// closed world shares with the JDK).
class Present
object Lazy:
  println("Lazy initialized")

object Main:
  def check(name: String): Unit =
    try println(Class.forName(name).getName)
    catch case _: ClassNotFoundException => println("missing " + name)
  def main(args: Array[String]): Unit =
    check("[LAbsent;")
    check("[[LAbsent;")
    check("[LPresent;")
    check("[[I")
    check("[LLazy$;")
    check("[L;")
    println("end")
