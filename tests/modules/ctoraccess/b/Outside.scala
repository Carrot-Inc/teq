package ctaout

// Outside `cta`: the qualified secondary is no alternative, and a protected constructor is reached by
// an anonymous subclass's creation.
object Outside:
  def hidden(): Any = new cta.Hidden("argument").x
  def anonymous(): Int = (new cta.Prot(11) {}).x + (new cta.P4(12) {}).x
  def upstream(): Int = cta.Made.anonymous().x
