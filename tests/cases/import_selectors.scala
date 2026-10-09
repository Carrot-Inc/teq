object Tools:
  val hammer: String = "hammer"
  val saw: String = "saw"
  val drill: String = "drill"
  def use(tool: String): String = s"using $tool"
  case class Box(items: List[String])

object Other:
  val saw: String = "other saw"
  val drill: String = "other drill"

object OldStyle:
  import Tools.{saw => _, drill => powerDrill, _}
  import Other._
  def run(): Unit =
    println(hammer)
    println(saw)
    println(powerDrill)
    println(drill)
    println(use(hammer))
    println(Box(List(powerDrill)))

object NewStyle:
  import Tools.{saw as _, drill as powerDrill, *}
  import Other.*
  def run(): Unit =
    println(saw)
    println(powerDrill)
    println(drill)

object Facade:
  export Tools.{saw => _, drill => powerDrill, _}

object Priority:
  import Other.*
  import Tools.saw
  def run(): Unit =
    println(saw)
    println(drill)

@main def main(): Unit =
  OldStyle.run()
  NewStyle.run()
  println(Facade.hammer)
  println(Facade.powerDrill)
  println(Facade.use("x"))
  Priority.run()
