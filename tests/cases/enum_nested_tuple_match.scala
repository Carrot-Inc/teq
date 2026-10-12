// teq: --werror
// An enum nested in a class has a companion per enclosing instance, its values read on it; a match
// over a tuple whose component is such a value covers the enum by its cases whatever the path, as
// dotty's `SpaceEngine` decomposes an enum by its children (`Symbol.children`): no warning, so the
// build passes under -Werror.
class Machine(val name: String):
  enum Phase:
    case Resting, Sending, Adding
    case Broken(reason: String)
  var phase: Phase = Phase.Resting
  def label(i: Int): String = (phase, i) match
    case (Phase.Resting, _) => "resting"
    case (Phase.Sending, n) => s"send $n"
    case (Phase.Adding, n) => s"add $n"
    case (Phase.Broken(r), _) => r
  def first(s: Phase, i: Int): String = (s, i) match
    case (Phase.Sending, 0) => "first send"
    case (Phase.Sending, _) => "send"
    case (Phase.Adding, _) => "adding"
    case (_, _) => "other"
  import Phase.*
  def short(s: Phase, i: Int): String = (s, i) match
    case (Resting, _) => s"$name resting"
    case (Sending, _) => s"$name send"
    case (Adding, _) => s"$name add"
    case (Broken(r), k) => s"$name $r $k"

class Plain:
  enum Mode:
    case On, Off
  def toggle(m: Mode, i: Int): Mode = (m, i) match
    case (Mode.On, _) => Mode.Off
    case (Mode.Off, _) => Mode.On

@main def run(): Unit =
  val m = new Machine("m")
  println(m.label(1))
  m.phase = m.Phase.Sending
  println(m.label(2))
  m.phase = m.Phase.Broken("worn")
  println(m.label(3))
  println(m.first(m.Phase.Sending, 0))
  println(m.first(m.Phase.Adding, 5))
  println(m.first(m.Phase.Resting, 5))
  println(m.short(m.Phase.Adding, 1))
  println(m.short(m.Phase.Broken("cut"), 7))
  val p = new Plain
  println(p.toggle(p.Mode.On, 0))
