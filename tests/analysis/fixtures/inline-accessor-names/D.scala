// A downstream of C.scala (tests/analysis-bytes.sh builds it over C.scala's products): its expansions of the inline
// methods read the accessors, whose names zinc gets in this class's used names (D.inc).
package q

object D:
  def use(c: p.C, f: p.F): Int =
    c.set(3)
    c.get + f.read + p.O.read
