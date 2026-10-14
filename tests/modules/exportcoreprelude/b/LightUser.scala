package xcore

case class LightUser(id: LightUser.Id)

object LightUser:
  opaque type Id = String
  object Id extends OpaqueString[Id]

@main def run(): Unit =
  val u = LightUser(LightUser.Id("ab"))
  println(u.id.value)
