package outside

// Outside `ctr` a qualified constructor is no alternative; a protected one is a super call's alone.
class D extends ctr.Prot(1):
  def body(): ctr.Prot = new ctr.Prot(2)
class KD extends ctr.K(1):
  def other(k: ctr.K): ctr.K = k.copy(x = 4)

object Use:
  def run(): Unit =
    println(new ctr.P3(1).x)
    println(new ctr.P4(2).x)
    println(new ctr.S().x)
    println(new ctr.S("s").x)
    println(ctr.K(3))
