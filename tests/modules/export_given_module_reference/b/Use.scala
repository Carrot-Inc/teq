package instanceclient
import exportedinstances.{Public, Render}
import Public.given
@main def run(): Unit =
  println(summon[Render[Int]].render(12))
