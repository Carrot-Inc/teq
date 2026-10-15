// Two bindings of one name that denote one thing, a package's alias of a class or a val declared as an
// object's singleton beside a wildcard import of the class or object itself, are no ambiguity (dotty's
// `checkNewOrShadowed` through `TypeComparer.isSameRef`), as indigo's package aliases and its imports meet.
package indigoish
object timeline:
  final case class TimelineAnimation[A](xs: List[A])
  object Timeline:
    def apply(n: Int): String = "timeline " + n
type TimelineAnimation[A] = timeline.TimelineAnimation[A]
val TimelineAnimation: timeline.TimelineAnimation.type = timeline.TimelineAnimation
val Timeline: timeline.Timeline.type = timeline.Timeline
object syntax:
  object animations:
    import indigoish.timeline.*
    def layer[A](xs: A*): TimelineAnimation[A] = TimelineAnimation(xs.toList)
    def tl(n: Int): String = Timeline(n)
@main def run(): Unit =
  println(syntax.animations.layer(1, 2))
  println(syntax.animations.tl(3))
