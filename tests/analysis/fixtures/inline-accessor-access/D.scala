package other
object D:
  def use(c: probe.C, f: probe.F): Int =
    c.set(1)
    f.set(2)
    probe.O.set(3)
    c.get + f.get + probe.O.get
