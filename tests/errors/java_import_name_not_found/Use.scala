// A single-type import whose qualifier holds no class of its name: the name is refused where it is used (E006), as
// the reference tells it; an unused one is no problem to the reference.
// expect: J.java:4:32: error: Not found: type Nope
object Use { def f = new p.J }
