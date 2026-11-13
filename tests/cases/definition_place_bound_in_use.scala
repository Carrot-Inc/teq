// A bound a case gave a type parameter is in use by a comparison when a member is first needed under it: the member
// is typed in its own place, where the bound is the case's as it was given and no comparison is using it.

// Expects "kept": `new Local` is held to `A` while `Local.extra`, an override of inferred result, is first needed;
// its body's overload reads the bound the case gave `A`. scalac prints kept.
object OneLevel:
  class Kept:
    def extra: Int = 0
  sealed trait Tag[A]
  case object KeptTag extends Tag[Kept { def extra: Int }]
  object Test:
    var seen = ""
    def which(x: Kept): String = "kept"
    def which(x: Any): String = "any"
    def run[A](tag: Tag[A]): String = tag match
      case KeptTag =>
        def caller: A = new Local
        class Local extends Kept:
          override def extra =
            seen = which(null.asInstanceOf[A])
            1
        caller.extra
        seen
    def main(args: Array[String]): Unit =
      println(run(KeptTag))

// Expects "holder / keeper holder": two bounds each in use by a comparison when a body is first needed under it.
// `new Local` is held to `A` while `Local.extra`, of inferred result, is first needed; its body holds `new Inner`
// to `B` while `Inner.more`, of inferred result, is first needed. In each body the overloads read the bounds the
// two cases gave. scalac prints holder / keeper holder.
object TwoLevels:
  class Holder:
    def extra: Int = 0
  class Keeper:
    def more: Int = 0
  sealed trait Tag[A]
  case object HTag extends Tag[Holder { def extra: Int }]
  sealed trait Mark[B]
  case object KMark extends Mark[Keeper { def more: Int }]
  object Test:
    var first = ""
    var seen = ""
    def which(x: Holder): String = "holder"
    def which(x: Any): String = "any"
    def what(x: Keeper): String = "keeper"
    def what(x: Any): String = "any"
    def run[A, B](tag: Tag[A], mark: Mark[B]): String = tag match
      case HTag => mark match
        case KMark =>
          def caller: A = new Local
          class Inner extends Keeper:
            override def more =
              seen = what(null.asInstanceOf[B]) + " " + which(null.asInstanceOf[A])
              1
          class Local extends Holder:
            override def extra =
              val inner: B = new Inner
              first = which(null.asInstanceOf[A])
              inner.more
          caller.extra
          first + " / " + seen
    def main(args: Array[String]): Unit =
      println(run(HTag, KMark))

@main def run(): Unit =
  OneLevel.Test.main(Array())
  TwoLevels.Test.main(Array())
