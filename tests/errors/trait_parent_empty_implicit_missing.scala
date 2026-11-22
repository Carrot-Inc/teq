// A plain list given to a trait's implicit clause is matched as the reference matches it (Applications' matchArgs): a
// parameter takes its written argument or its default; the first that has neither is E171, the method type being
// applied shown from that clause on, with the migration line for an empty list (`failEmptyArgs`) and without it for a
// short one. By a class parent and an anonymous class: defaults on some parameters only, an implicit clause after a
// using one, a short list.
// expect: 26:18: error: missing argument for parameter b of constructor T2 in trait T2: (implicit a: Int, b: String): T2
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect: 28:18: error: missing argument for parameter b of constructor TU in trait TU: (implicit b: B, c: C): TU
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect: 30:18: error: missing argument for parameter b of constructor TS in trait TS: (implicit a: A, b: B): TS
// expect: 32:18: error: missing argument for parameter c of constructor TV in trait TV: (implicit b: B, c: C): TV
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect: 34:16: error: missing argument for parameter b of constructor T2 in trait T2: (implicit a: Int, b: String): T2
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect: 35:16: error: missing argument for parameter b of constructor TU in trait TU: (implicit b: B, c: C): TU
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect: 36:16: error: missing argument for parameter b of constructor TS in trait TS: (implicit a: A, b: B): TS
// expect: 37:16: error: missing argument for parameter c of constructor TV in trait TV: (implicit b: B, c: C): TV
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
class A(val s: String)
class B(val s: String)
class C(val s: String)
given Int = 7
given B = B("gb")
trait T2(implicit a: Int = 45, b: String)
class P1 extends T2()()
trait TU(using a: A)(implicit b: B, c: C)
class P2 extends TU(using A("wa"))()
trait TS(implicit a: A, b: B)
class P3 extends TS()(A("wa"))
trait TV(using a: A)(implicit b: B = B("db"), c: C)
class P4 extends TV(using A("wa"))()
object Q:
  val q1 = new T2()() {}
  val q2 = new TU(using A("wa"))() {}
  val q3 = new TS()(A("wa")) {}
  val q4 = new TV(using A("wa"))() {}
