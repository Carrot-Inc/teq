// A package's val declared as an object's singleton (indigo's `val Timeline: anim.Timeline.type = anim.Timeline`, written
// after its use) and an import of the object itself are one binding (`TypeComparer.isSameRef`), and an export through
// the val forwards the object's members, as `Namer` types an export's qualifier and forwards through its type.
package eng

object animations:
  import eng.anim.*

  def timeline[A](items: A*): Timeline[A] =
    Timeline(items.toList)

  export Timeline.start

type Timeline[A] = eng.anim.Timeline[A]
val Timeline: eng.anim.Timeline.type = eng.anim.Timeline

@main def run(): Unit =
  println(animations.timeline(1, 2, 3).size)
  println(animations.start)
