package sbt.internal.inc

/** The names zinc stored as each class's used names, which its package keeps to itself. */
object StoredNames {
  def of(analysis: Analysis): Map[String, Set[(String, Set[String])]] =
    analysis.relations.names.iterator.map{ case (c, used) => c -> used.map(u => (u.name, u.scopes.toArray.map(_.toString).toSet)).toSet}.toMap
}
