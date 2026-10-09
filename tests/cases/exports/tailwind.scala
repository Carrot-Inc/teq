package sharedlib

object Tailwind:
  case class Tw(classes: List[String]):
    def render: String = classes.mkString(" ")

  val tw: Tw = Tw(Nil)

  def cls(names: String*): Tw = Tw(names.toList)

  def classNames(pairs: (String, Boolean)*): Tw = Tw(pairs.toList.filter(_._2).map(_._1))

  def notExported: Int = 0
