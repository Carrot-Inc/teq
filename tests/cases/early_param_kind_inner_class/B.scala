// The enclosing trait and class of early_param_kind_inner_class's other file, each with an inner class whose
// higher-kinded parameter is bounded by a member or a type parameter of the enclosing one.
package earlykindprefix

trait Stock { class Item; class Crate[X] extends Item; class Rack[F[X] <: Item] }
class Load
class LoadOf[X] extends Load
class Yard[T] { class Slot[F[X] <: T] }
