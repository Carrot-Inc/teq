//! The departures from dotty 3.8.4 this part leaves to other parts, each pinned by the answer scalac 3.8.4 gives
//! (its typed tree, out/fix-semantic/pin-probes) and ignored until the part named in its name lands: there the pin
//! is run, and passes or names what that part's port lacks. A departure a program's output shows is pinned by the
//! program instead (tests/scala3-typing/proposed-errors for one scalac refuses); these are the ones no output shows,
//! a type of teq's alone. A type is read as `teq fix`'s forms view writes it (`retained_view`): under semantic typing
//! with its retained forms, else the canonical type.

use super::*;
use crate::types::Retention;

/// The type of `owner.name` in the typed program, as the forms view writes it.
fn view(w: &mut Worker, owner: &str, name: &str) -> String {
    use crate::symbols::{Owner, SymKind};
    let s = (0..w.syms.syms.len() as u32)
        .map(crate::types::SymId)
        .find(|&s| {
            let info = w.syms.sym(s);
            matches!(info.kind, SymKind::Val | SymKind::Def)
                && w.name_ref(info.name) == name
                && matches!(info.owner, Owner::Class(c) if w.name_ref(w.syms.class(c).name) == owner)
        })
        .unwrap_or_else(|| panic!("no member {}.{}", owner, name));
    let file = w.syms.sym(s).file;
    let lines = super::semanticdb::retained_view(w, &[file]);
    let head = format!("{}.{}: ", owner, name);
    lines.lines().find_map(|l| l.strip_prefix(head.as_str())).map(str::to_string).unwrap_or_else(|| panic!("no view of {}.{}", owner, name))
}

fn alias(w: &mut Worker, n: &str) -> AliasId {
    let a = (0..w.syms.aliases.len()).map(|i| AliasId(i as u32)).find(|&a| w.name_ref(w.syms.aliases[a.idx()].name) == n).expect("the alias");
    w.complete_alias(a);
    a
}

fn class(w: &Worker, n: &str) -> ClassId {
    (0..w.syms.classes.len()).map(|i| ClassId(i as u32)).find(|&c| w.name_ref(w.syms.class(c).name) == n).expect("the class")
}

/// `Variances.setStructuralVariances` over `TypeAccumulator.foldArgs` (the pass over c02b18202's 3): a parameter
/// applied to a higher-kinded one is read at that one's variance (`type HK[F[+_], A] = F[A]`: `[1, 1]`), and an alias
/// without parameters of its own reads its right side's (`type L = List` applied is covariant, `type T = [X] =>>
/// In[X]` applied contravariant). Carried from the dry rebase's scratch (`an_alias_without_parameters_reads_its_right_
/// sides`, source-version-3's finding 2, and the variances test's `HK`).
#[test]
#[ignore = "waits for source-version-3's setStructuralVariances port"]
fn source_version_3_structural_variances_through_a_higher_kinded_argument_and_an_alias_without_parameters() {
    const PROGRAM: &str = "class In[-A]\nobject O:\n  type HK[F[+_], A] = F[A]\n  type L = List\n  type T = [X] =>> In[X]\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-structural-variances", PROGRAM, &[], |w| {
        let hk = alias(w, "HK");
        assert_eq!(w.alias_param_signs(hk), vec![1, 1], "HK[F[+_], A]");
        let int = w.b.t_int;
        let args = w.types.list(&[int]);
        for (n, sign) in [("L", 1), ("T", -1)] {
            let a = alias(w, n);
            let r = w.types.retain(Retention::Alias(a, args));
            assert_eq!(w.form_parts_at(r), vec![(int, sign)], "{}[Int]", n);
        }
    });
}

/// `TypeApplications.typeParams` following a parameter's bound through another parameter (the pass over c02b18202's
/// 4): in `class G[F[_], H <: F]` both compilers accept, `H.etaExpand` is a lambda of one parameter, `F`'s. Carried
/// from the dry rebase's scratch (`eta_expansion_of_a_parameter_or_member_by_its_type_params`, source-version-3's
/// finding 1).
#[test]
#[ignore = "waits for source-version-3's typeParams and etaExpand port"]
fn source_version_3_eta_expands_a_parameter_bounded_by_another() {
    const PROGRAM: &str = "class G[F[_], H <: F]\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-eta-bound", PROGRAM, &[], |w| {
        let g = class(w, "G");
        let h = w.syms.class(g).tparams[1];
        let ht = w.types.param(h);
        let (l, _) = w.eta_expand(ht).expect("H eta-expanded");
        assert_eq!(w.types.items(l).len(), 1, "H's type parameters are F's");
    });
}

/// `ApproximatingTypeMap.derivedAppliedType` over an alias whose argument escapes (section 14's question 2): scalac
/// keeps the alias with the range distributed to its ends, `Box[? >: K[Nothing] <: K[Any]]`, where teq's canonical
/// alias maps its argument at the caller's variance, `Box[K[?]]`.
#[test]
#[ignore = "waits for source-version-3's ApproximatingTypeMap port"]
fn source_version_3_distributes_an_alias_s_escaping_argument_to_its_ends() {
    const PROGRAM: &str = "class C { type T }\nclass Box[A]\nobject Pins:\n  type K[A] = Int\n  def map[A, B](a: A)(f: A => B): B = f(a)\n  val boxK = map(new C)(x => (??? : Box[K[x.T]]))\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-alias-ends", PROGRAM, &[], |w| {
        assert_eq!(view(w, "Pins", "boxK"), "Box[? >: K[Nothing] <: K[Any]]");
    });
}

/// `Annotation.mapWith` over an annotation's whole typed tree, its type trees included (the pass over c02b18202's 1,
/// c14fd75e1's 2): an argument naming an escaping path in a type argument, a type test and a local type definition
/// approximates the whole annotated type, `Box[?]`, where teq's layer keeps `Box[? <: Any @Ann]`.
#[test]
#[ignore = "waits for type-representation's AnnotatedType"]
fn type_representation_maps_an_annotation_s_type_tree_arguments() {
    const PROGRAM: &str = "import scala.annotation.StaticAnnotation\nclass Ann(val x: Any) extends StaticAnnotation\nclass Box[A]\nclass C { type T }\nobject AnnTrees {\n  def map[X, Y](a: X)(k: X => Y): Y = k(a)\n  def id[A]: Int = 1\n  val typeArg = map(new C)(x => (??? : Box[x.T @Ann(id[x.T])]))\n  val typeTest = map(new C)(x => (??? : Box[x.T @Ann(1.isInstanceOf[x.T])]))\n  val localType = map(new C)(x => (??? : Box[x.T @Ann({ type L = x.T; 1 })]))\n}\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-annotation-trees", PROGRAM, &["--semantic-types"], |w| {
        for name in ["typeArg", "typeTest", "localType"] {
            assert_eq!(view(w, "AnnTrees", name), "Box[?]", "{}", name);
        }
    });
}

/// `Annotations.annotClass` through a generic inner class selected through a path (`Types.typeSymbol` of a prefixed
/// `TypeRef`; the pass over 32fc738c3's 1): `Box[Int @Ann]`, where teq's `Lambda` over a `Nested` drops it.
#[test]
#[ignore = "waits for type-representation's TypeRef"]
fn type_representation_reads_an_annotation_class_through_a_prefixed_type_ref() {
    const PROGRAM: &str = "class Box[A]\nclass Outer:\n  class Ann[A] extends scala.annotation.StaticAnnotation\n  def inner: Box[Int @Ann[String]] = ???\nobject GenericAnn:\n  val outer: Outer = new Outer\n  def qualified: Box[Int @outer.Ann[String]] = ???\n  val a = outer.inner\n  val b = qualified\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-generic-annotation", PROGRAM, &["--semantic-types"], |w| {
        assert_eq!(view(w, "GenericAnn", "qualified"), "Box[Int @Ann]");
        assert_eq!(view(w, "GenericAnn", "b"), "Box[Int @Ann]");
    });
}

/// `TypeOps.simplify` over an annotated bottom type and `Annotation.mapWith` over an unstable argument's path (the
/// pass over c02b18202's 6; section 17): `Box[Nothing]` where teq keeps `Box[Nothing @Mark]`, and `Box[Int]` at
/// `f(new C)` where teq keeps `Box[Int @Ann]`. A loaded annotation's arguments are not read either (the loader keeps
/// its class alone), so the same call against a library's classes gives the same.
#[test]
#[ignore = "waits for type-representation's AnnotatedType"]
fn type_representation_drops_an_annotation_over_nothing_and_over_an_unstable_argument() {
    const PROGRAM: &str = "import scala.annotation.StaticAnnotation\nclass Ann(val x: Any) extends StaticAnnotation\nclass Mark extends StaticAnnotation\nclass C { type T }\nclass Box[A]\nobject Pins:\n  def map[A, B](a: A)(f: A => B): B = f(a)\n  def f(t: C): Box[Int @Ann(t)] = ???\n  val annArg = f(new C)\n  val bottom = map(new C)(x => (??? : Box[(Nothing & String & x.T) @Mark]))\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-annotation-drops", PROGRAM, &["--semantic-types"], |w| {
        assert_eq!(view(w, "Pins", "annArg"), "Box[Int]");
        assert_eq!(view(w, "Pins", "bottom"), "Box[Nothing]");
    });
}

/// `TypeComparer.firstTry`'s and `secondTry`'s `NamedType` alias arms (the pass over 8b8caaa92's 2 and 32fc738c3's
/// 2): an alias's `TypeRef` is dealiased before the annotation arms and the constraint, so a parameter meeting
/// `Marked = Int @Ann`, an alias of it and a plain alias is bounded by `Int`.
#[test]
#[ignore = "waits for type-representation's TypeRef"]
fn type_representation_dealiases_an_alias_reference_before_the_constraint() {
    const PROGRAM: &str = "class Ann extends scala.annotation.StaticAnnotation\nclass In[-A]\nclass Out[+A]\nobject AliasBound:\n  type Marked = Int @Ann\n  type Plain = Int\n  type Double = Marked\n  def take[A](x: In[A]): In[A] = x\n  def pick[A](x: Out[A]): Out[A] = x\n  val aliased = take(??? : In[Marked])\n  val twice = take(??? : In[Double])\n  val plain = take(??? : In[Plain])\n  val lower = pick(??? : Out[Marked])\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-alias-bound", PROGRAM, &["--semantic-types"], |w| {
        for name in ["aliased", "twice", "plain"] {
            assert_eq!(view(w, "AliasBound", name), "In[Int]", "{}", name);
        }
        assert_eq!(view(w, "AliasBound", "lower"), "Out[Int]");
    });
}

/// A module's `TermRef` as the type of an object referenced as a value: `Obj.type` (`SingleType` in the documents),
/// where teq types the term as its module class's type.
#[test]
#[ignore = "waits for type-representation's TermRef"]
fn type_representation_types_an_object_value_as_its_term_ref() {
    const PROGRAM: &str = "class Out[+A]\nobject Obj\nobject ModuleRef:\n  def id[A](a: A): A = a\n  def wrap[A](a: A): Out[A] = ???\n  val plain = Obj\n  val single = id(Obj)\n  val wrapped = wrap(Obj)\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-module-term-ref", PROGRAM, &["--semantic-types"], |w| {
        assert_eq!(view(w, "ModuleRef", "plain"), "Obj.type");
        assert_eq!(view(w, "ModuleRef", "single"), "Obj.type");
        assert_eq!(view(w, "ModuleRef", "wrapped"), "Out[Obj.type]");
    });
}

/// A dependent method's result over an unstable argument (`TypeAssigner.assignType(Apply)` with the argument's
/// skolem, approximated): `Inv[? >: CharSequence]`, where teq gives `Inv[C#T | CharSequence]`.
#[test]
#[ignore = "waits for parallel-retype-2's T6"]
fn parallel_retype_2_approximates_a_dependent_result_over_an_unstable_argument() {
    const PROGRAM: &str = "class C { type T }\nclass Inv[A]\nobject Pins:\n  def gu(x: C): Inv[x.T | String | CharSequence] = ???\n  val depUnion = gu(new C)\n@main def run(): Unit = println(1)\n";
    crate::with_typed_program("pin-dependent-union", PROGRAM, &[], |w| {
        assert_eq!(view(w, "Pins", "depUnion"), "Inv[? >: CharSequence]");
    });
}
