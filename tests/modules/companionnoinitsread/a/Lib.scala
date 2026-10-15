package cnr

// Written companions whose bodies hold a given alias and a nested object: an initialiser in the
// source's judgement (`TreeInfo.defKind`: a val with a right-hand side), none in their pickles'
// (`TreeUnpickler.indexStats`: the given's val and the module's are lazy).
object Models:
  final case class G(v: Int)
  object G:
    given Ordering[G] = Ordering.by(_.v)
  final case class N(v: Int)
  object N:
    object Inner
    def f = 1
