// Adapted from EmptyCollectionsUnified of scalafix-unified (https://github.com/ghostbuster91/scalafix-unified, commit
// 6563e96c9f0b26175c226b758c752ce1123249ae, rules/src/main/scala/fix/EmptyCollectionsUnified.scala), under the
// Apache License, Version 2.0 as the repository's build.sbt declares it (the repository has no licence file and its
// files name no copyright holder): http://www.apache.org/licenses/LICENSE-2.0
//
// The port's local change: the reference's quasiquotes are written as the patterns they stand for
// (`q"List[$e]()"` is `Term.Apply(Term.ApplyType(Term.Name("List"), List(e)), Nil)`, `p"List()"` is
// `Pat.Extract(Term.Name("List"), Nil)`), matched in the reference's order.
package teq.fix.rules

import teq.fix.*

final class EmptyCollectionsUnified extends SemanticRule("EmptyCollectionsUnified"):
  override def fix(using doc: SemanticDocument): Patch =
    doc.tree.collect {
      case t @ Term.Apply(Term.ApplyType(Term.Name("List"), List(e)), Nil) => Patch.replaceTree(t, "List.empty[" + e.syntax + "]").atomic
      case t @ Term.Name("List") =>
        t.parent match
          // val b: List[Int] = List()
          case Some(p @ Term.Apply(_, Nil)) => Patch.replaceTree(p, "List.empty").atomic
          //  case (List(), List()) =>
          //  case List() =>
          case Some(p @ Pat.Extract(Term.Name("List"), Nil)) => Patch.replaceTree(p, "Nil").atomic
          case _ => Patch.empty
      case t @ Term.Name("Nil") =>
        t.parent match
          // ::(_, Nil)
          case Some(Term.Apply(Term.Name("::"), _)) => Patch.empty
          // 1 :: Nil
          case Some(Term.ApplyInfix(_, Term.Name("::"), _, _)) => Patch.empty
          // case Nil =>
          case Some(p) if EmptyCollectionsUnified.insidePatternMatching(p) => Patch.empty
          case _ => Patch.replaceTree(t, "List.empty").atomic
      case t @ Term.Apply(Term.Name("Set"), Nil) => Patch.replaceTree(t, "Set.empty").atomic
      case t @ Term.Apply(Term.ApplyType(Term.Name("Set"), List(e)), Nil) => Patch.replaceTree(t, "Set.empty[" + e.syntax + "]").atomic
      case t @ Term.Apply(Term.Name("Map"), Nil) => Patch.replaceTree(t, "Map.empty").atomic
      case t @ Term.Apply(Term.ApplyType(Term.Name("Map"), List(k, v)), Nil) => Patch.replaceTree(t, "Map.empty[" + k.syntax + ", " + v.syntax + "]").atomic
      case _ => Patch.empty
    }.asPatch

object EmptyCollectionsUnified:
  def insidePatternMatching(t: Tree): Boolean = t match
    case _: Case => true
    case _: Pat.Extract => true
    case _: Pat.ExtractInfix => true
    case _ => t.parent.fold(false)(insidePatternMatching)
