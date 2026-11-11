import holders.given
@main def run(): Unit =
  println(summon[Int])
  println(summon[String])
  println(inspection.holderIs[Int]("One$package"))
  println(inspection.holderIs[String]("Two$package"))
