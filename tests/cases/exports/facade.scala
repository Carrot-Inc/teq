package facade

object IconSetGenerated:
  def home: String = "<svg home/>"
  def user: String = "<svg user/>"

object IconSet:
  export IconSetGenerated.*

  case class IconDef(name: String, size: Int)

  def icon(name: String): IconDef = IconDef(name, 16)

  def homeTwice: String = home + home
