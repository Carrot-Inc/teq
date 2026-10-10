package sbt.internal.teq

import java.util.IdentityHashMap

import scala.collection.mutable
import scala.collection.compat.*

import dev.teq.sbt.Json
import xsbti.api.*

/** teq's API graph (`--analysis-version 2`, `docs/TARGETS.md`, "The analysis graph") as zinc's
  * `xsbti.api` objects, and those objects as a graph again.
  *
  * An answer's graph is `{"nodes": [...], "files": [{"file", "classes", "products", "local"}]}`:
  * `nodes` holds the types, the definitions and the class-likes of every file, each an array
  * whose first element names the `xsbti.api` class it stands for and whose other elements are
  * that class's fields in the order of its factory method, a node standing in a field by its
  * index; per file, `classes` lists the indices of the `ClassLike`s zinc's callback receives,
  * `products` each class's name with the binary name of its class file, `local` the names of
  * the file's local classes, which have no API. An access, a modifier set, an annotation, a type
  * parameter and a parameter list sit inline. A node referenced twice is one object: zinc's
  * hashing visits a `Structure` or a `ClassLike` once by identity, so what the compiler shares
  * is part of what it says (a class's hashing sees its own file's nodes alone, so one table for
  * every file shares nothing a file's own would not). The construction is mechanical: no type is
  * parsed, nothing is normalised, and zinc computes the hashes from the objects.
  *
  * The writer is the reader's inverse, for the comparison with scalac's analysis and for the
  * tests: `Structure`s and `ClassLike`s by identity, every other node by value. */
object ApiGraph {
  final class Malformed(message: String) extends Exception(message)

  /** The objects of one file's graph, built as they are asked for; a `Structure`'s members and a
    * `ClassLike`'s structure and self type are lazy, so that a graph may refer to itself. Every
    * field is read as the type it must have: a field of another type, a missing field or one
    * too many refuses the graph rather than read as empty. */
  final class Reader(nodes: IndexedSeq[Json.Value]) {
    private val built = new Array[AnyRef](nodes.length)
    /** The nodes being built, which a node built eagerly may not refer to: only a structure's
      * members and a class's structure and self type, lazy, may lead back to their node. */
    private val building = new Array[Boolean](nodes.length)

    def classLike(i: Int): ClassLike = node(i) match {
      case c: ClassLike => c
      case other => fail(s"node $i is not a ClassLike")
    }

    def node(i: Int): AnyRef = {
      if (i < 0 || i >= nodes.length) fail(s"no node $i")
      if (built(i) == null) {
        if (building(i)) fail(s"node $i refers to itself other than through a structure")
        building(i) = true
        built(i) = make(i, array(nodes(i), s"node $i"))
        building(i) = false
      }
      built(i)
    }

    /** Builds every node and what each lazy field refers to, so that a graph that refers to a
      * node it has not, or to one of the wrong kind, is refused before it is used. */
    def buildAll(): Unit =
      for (i <- nodes.indices)
        node(i) match {
          case s: Structure =>
            s.parents
            s.declared
            s.inherited
          case c: ClassLike =>
            c.selfType
            c.structure
          case _ => ()
        }

    private def fail(what: String): Nothing = throw new Malformed(what)

    private def int(v: Json.Value, what: => String): Int = v match {
      case Json.Num(n) => n.toIntOption.getOrElse(fail(s"$what: $n is not an integer"))
      case _ => fail(s"$what is not a number")
    }

    private def tpe(v: Json.Value, what: => String): Type = {
      val i = int(v, what)
      node(i) match {
        case t: Type => t
        case _ => fail(s"$what: node $i is not a type")
      }
    }

    private def definition(v: Json.Value, what: => String): ClassDefinition = {
      val i = int(v, what)
      node(i) match {
        case d: ClassDefinition => d
        case _ => fail(s"$what: node $i is not a definition")
      }
    }

    /** The number of fields of each kind of node, its kind among them. */
    private val arity = Map(
      "EmptyType" -> 1, "Projection" -> 3, "ParameterRef" -> 2, "Singleton" -> 2, "Parameterized" -> 3,
      "Constant" -> 3, "Annotated" -> 3, "Structure" -> 4, "Existential" -> 3, "Polymorphic" -> 3,
      "Val" -> 6, "Var" -> 6, "Def" -> 8, "TypeAlias" -> 7, "TypeDeclaration" -> 8, "ClassLikeDef" -> 7,
      "ClassLike" -> 12,
    )

    private def make(i: Int, fields: Seq[Json.Value]): AnyRef = {
      if (fields.isEmpty) fail(s"node $i is empty")
      val kind = string(fields.head, s"node $i's kind")
      arity.get(kind) match {
        case Some(n) if n == fields.length => ()
        case Some(n) => fail(s"node $i ($kind) has ${fields.length} fields, not $n")
        case None => fail(s"unknown node $kind")
      }
      def what(k: Int) = s"node $i ($kind) field $k"
      def f(k: Int): Json.Value = fields(k)
      def str(k: Int): String = string(f(k), what(k))
      def t(k: Int): Type = tpe(f(k), what(k))
      def arr(k: Int): Seq[Json.Value] = array(f(k), what(k))
      kind match {
        case "EmptyType" => EmptyType.of()
        case "Projection" => Projection.of(t(1), str(2))
        case "ParameterRef" => ParameterRef.of(str(1))
        case "Singleton" => Singleton.of(path(f(1), what(1)))
        case "Parameterized" => Parameterized.of(t(1), arr(2).map(tpe(_, what(2))).toArray)
        case "Constant" => Constant.of(t(1), str(2))
        case "Annotated" => Annotated.of(t(1), arr(2).map(annotation(_, what(2))).toArray)
        case "Structure" =>
          val (parents, declared, inherited) = (arr(1), arr(2), arr(3))
          Structure.of(
            lazily(parents.map(tpe(_, what(1))).toArray),
            lazily(declared.map(definition(_, what(2))).toArray),
            lazily(inherited.map(definition(_, what(3))).toArray),
          )
        case "Existential" => Existential.of(t(1), arr(2).map(typeParameter(_, what(2))).toArray)
        case "Polymorphic" => Polymorphic.of(t(1), arr(2).map(typeParameter(_, what(2))).toArray)
        case "Val" => Val.of(str(1), access(f(2), what(2)), modifiers(f(3), what(3)), annotations(f(4), what(4)), t(5))
        case "Var" => Var.of(str(1), access(f(2), what(2)), modifiers(f(3), what(3)), annotations(f(4), what(4)), t(5))
        case "Def" =>
          Def.of(str(1), access(f(2), what(2)), modifiers(f(3), what(3)), annotations(f(4), what(4)), typeParameters(f(5), what(5)),
            arr(6).map(parameterList(_, what(6))).toArray, t(7))
        case "TypeAlias" => TypeAlias.of(str(1), access(f(2), what(2)), modifiers(f(3), what(3)), annotations(f(4), what(4)), typeParameters(f(5), what(5)), t(6))
        case "TypeDeclaration" =>
          TypeDeclaration.of(str(1), access(f(2), what(2)), modifiers(f(3), what(3)), annotations(f(4), what(4)), typeParameters(f(5), what(5)), t(6), t(7))
        case "ClassLikeDef" =>
          ClassLikeDef.of(str(1), access(f(2), what(2)), modifiers(f(3), what(3)), annotations(f(4), what(4)), typeParameters(f(5), what(5)), definitionType(f(6), what(6)))
        case "ClassLike" =>
          val (self, structure) = (f(6), f(7))
          int(self, what(6))
          val s = int(structure, what(7))
          ClassLike.of(
            str(1),
            access(f(2), what(2)),
            modifiers(f(3), what(3)),
            annotations(f(4), what(4)),
            definitionType(f(5), what(5)),
            lazily(tpe(self, what(6))),
            lazily(node(s) match {
              case st: Structure => st
              case _ => fail(s"${what(7)}: node $s is not a Structure")
            }),
            arr(8).map(string(_, what(8))).toArray,
            arr(9).map(tpe(_, what(9))).toArray,
            boolean(f(10), what(10)),
            typeParameters(f(11), what(11)),
          )
      }
    }

    private def path(v: Json.Value, what: => String): Path = Path.of(array(v, what).map(pathComponent(_, what)).toArray)

    private def pathComponent(v: Json.Value, what: => String): PathComponent = v match {
      case Json.Arr(Seq(Json.Str("Id"), Json.Str(id))) => Id.of(id)
      case Json.Arr(Seq(Json.Str("This"))) => This.of()
      case Json.Arr(Seq(Json.Str("Super"), qualifier)) => Super.of(path(qualifier, what))
      case _ => fail(s"$what: not a path component: $v")
    }

    private def access(v: Json.Value, what: => String): Access = v match {
      case Json.Str("Public") => Public.of()
      case Json.Arr(Seq(Json.Str("Private"), q)) => Private.of(qualifier(q, what))
      case Json.Arr(Seq(Json.Str("Protected"), q)) => Protected.of(qualifier(q, what))
      case _ => fail(s"$what: not an access: $v")
    }

    private def qualifier(v: Json.Value, what: => String): Qualifier = v match {
      case Json.Str("Unqualified") => Unqualified.of()
      case Json.Str("This") => ThisQualifier.of()
      case Json.Arr(Seq(Json.Str("Id"), Json.Str(id))) => IdQualifier.of(id)
      case _ => fail(s"$what: not a qualifier: $v")
    }

    private def modifiers(v: Json.Value, what: => String): Modifiers = {
      val b = int(v, what)
      if (b < 0 || b > 255) fail(s"$what: $b is not a modifier set")
      def bit(i: Int) = (b & (1 << i)) != 0
      new Modifiers(bit(0), bit(1), bit(2), bit(3), bit(4), bit(5), bit(6), bit(7))
    }

    private def annotations(v: Json.Value, what: => String): Array[Annotation] = array(v, what).map(annotation(_, what)).toArray

    private def annotation(v: Json.Value, what: => String): Annotation = v match {
      case Json.Arr(Seq(base, args)) =>
        val arguments = array(args, what).map {
          case Json.Arr(Seq(Json.Str(name), Json.Str(value))) => AnnotationArgument.of(name, value)
          case other => fail(s"$what: not an annotation argument: $other")
        }
        Annotation.of(tpe(base, what), arguments.toArray)
      case _ => fail(s"$what: not an annotation: $v")
    }

    private def typeParameters(v: Json.Value, what: => String): Array[TypeParameter] = array(v, what).map(typeParameter(_, what)).toArray

    private def typeParameter(v: Json.Value, what: => String): TypeParameter = v match {
      case Json.Arr(Seq(id, annots, tparams, variance, lo, hi)) =>
        TypeParameter.of(string(id, what), annotations(annots, what), typeParameters(tparams, what), enumValue(variance, what)(Variance.valueOf), tpe(lo, what), tpe(hi, what))
      case _ => fail(s"$what: not a type parameter: $v")
    }

    private def parameterList(v: Json.Value, what: => String): ParameterList = v match {
      case Json.Arr(Seq(params, isImplicit)) => ParameterList.of(array(params, what).map(parameter(_, what)).toArray, boolean(isImplicit, what))
      case _ => fail(s"$what: not a parameter list: $v")
    }

    private def parameter(v: Json.Value, what: => String): MethodParameter = v match {
      case Json.Arr(Seq(name, t, hasDefault, modifier)) =>
        MethodParameter.of(string(name, what), tpe(t, what), boolean(hasDefault, what), enumValue(modifier, what)(ParameterModifier.valueOf))
      case _ => fail(s"$what: not a parameter: $v")
    }

    private def definitionType(v: Json.Value, what: => String): DefinitionType = enumValue(v, what)(DefinitionType.valueOf)

    private def enumValue[T](v: Json.Value, what: => String)(valueOf: String => T): T = {
      val name = string(v, what)
      try valueOf(name)
      catch { case _: IllegalArgumentException => fail(s"$what: $name is not a constant of its enum") }
    }
  }

  /** One file of a graph: its `ClassLike`s in the order of `classes`, each class's name with the
    * binary name of its class file, and its local classes' names. */
  final case class File(path: String, classes: Seq[ClassLike], products: Seq[(String, String)], local: Seq[String])

  /** `v` as an array, a string or a boolean, refused as `what` otherwise. */
  def array(v: Json.Value, what: => String): Seq[Json.Value] = v match {
    case Json.Arr(items) => items
    case _ => throw new Malformed(s"$what is not an array")
  }
  def string(v: Json.Value, what: => String): String = v match {
    case Json.Str(s) => s
    case _ => throw new Malformed(s"$what is not a string")
  }
  def boolean(v: Json.Value, what: => String): Boolean = v match {
    case Json.Bool(b) => b
    case _ => throw new Malformed(s"$what is not a boolean")
  }

  /** The files of an answer's graph, their objects built from its one table of nodes. */
  def read(api: Json.Value): Seq[File] = {
    val reader = new Reader(array(api("nodes"), "the graph's nodes").toIndexedSeq)
    val files = array(api("files"), "the graph's files").map { f =>
      val path = string(f("file"), "a file's name")
      val classes = array(f("classes"), s"$path's classes").map {
        case Json.Num(n) => reader.classLike(n.toIntOption.getOrElse(throw new Malformed(s"$path: $n is not a node")))
        case other => throw new Malformed(s"$path: a class is not a node's index")
      }
      val products = array(f("products"), s"$path's products").map { p =>
        array(p, s"a product of $path") match {
          case Seq(name, binary) => (string(name, s"a product's class in $path"), string(binary, s"a product's binary name in $path"))
          case _ => throw new Malformed(s"a product of $path is not a class and a binary name")
        }
      }
      val local = array(f("local"), s"$path's local classes").map(string(_, s"a local class of $path"))
      File(path, classes, products, local)
    }
    reader.buildAll()
    files
  }

  /** The graph of `files` as an answer's JSON. */
  def write(files: Seq[File]): String = {
    val w = new Writer
    val entries = files.map { f =>
      val roots = f.classes.map(w.index)
      val products = f.products.map{ case (n, b) => s"[${Json.string(n)},${Json.string(b)}]"}.mkString(",")
      s"""{"file":${Json.string(f.path)},"classes":[${roots.mkString(",")}],"products":[$products],"local":[${f.local.map(Json.string).mkString(",")}]}"""
    }
    val out = new StringBuilder
    out ++= "{\"nodes\":["
    w.nodes.zipWithIndex.foreach { case (n, i) =>
      if (i > 0) out += ','
      out ++= n
    }
    out ++= "],\"files\":[" ++= entries.mkString(",") ++= "]}"
    out.result()
  }

  private final class Writer {
    val nodes = mutable.ArrayBuffer.empty[String]
    private val byValue = mutable.HashMap.empty[String, Int]
    private val byIdentity = new IdentityHashMap[AnyRef, Integer]

    def index(n: AnyRef): Int = n match {
      case s: Structure => identified(s)(structure(s))
      case c: ClassLike => identified(c)(classLike(c))
      case other => valued(render(other))
    }

    private def identified(n: AnyRef)(text: => String): Int = {
      val known = byIdentity.get(n)
      if (known != null) known.intValue
      else {
        val i = nodes.length
        nodes += ""
        byIdentity.put(n, i)
        nodes(i) = text
        i
      }
    }

    private def valued(text: String): Int = byValue.getOrElseUpdate(text, { nodes += text; nodes.length - 1 })

    private def arr(items: Iterable[String]): String = items.mkString("[", ",", "]")
    private def ref(n: AnyRef): String = index(n).toString
    private def refs(ns: Array[? <: AnyRef]): String = arr(ns.map(ref))
    private def str(s: String): String = Json.string(s)

    private def structure(s: Structure): String =
      arr(Seq(str("Structure"), refs(s.parents), refs(s.declared), refs(s.inherited)))

    private def classLike(c: ClassLike): String =
      arr(Seq(
        str("ClassLike"), str(c.name), access(c.access), modifiers(c.modifiers), annotations(c.annotations),
        str(c.definitionType.name), ref(c.selfType), ref(c.structure), arr(c.savedAnnotations.map(str)),
        refs(c.childrenOfSealedClass), c.topLevel.toString, typeParameters(c.typeParameters),
      ))

    private def render(n: AnyRef): String = n match {
      case _: EmptyType => arr(Seq(str("EmptyType")))
      case t: Projection => arr(Seq(str("Projection"), ref(t.prefix), str(t.id)))
      case t: ParameterRef => arr(Seq(str("ParameterRef"), str(t.id)))
      case t: Singleton => arr(Seq(str("Singleton"), path(t.path)))
      case t: Parameterized => arr(Seq(str("Parameterized"), ref(t.baseType), refs(t.typeArguments)))
      case t: Constant => arr(Seq(str("Constant"), ref(t.baseType), str(t.value)))
      case t: Annotated => arr(Seq(str("Annotated"), ref(t.baseType), annotations(t.annotations)))
      case t: Existential => arr(Seq(str("Existential"), ref(t.baseType), typeParameters(t.clause)))
      case t: Polymorphic => arr(Seq(str("Polymorphic"), ref(t.baseType), typeParameters(t.parameters)))
      case d: Val => arr(Seq(str("Val"), str(d.name), access(d.access), modifiers(d.modifiers), annotations(d.annotations), ref(d.tpe)))
      case d: Var => arr(Seq(str("Var"), str(d.name), access(d.access), modifiers(d.modifiers), annotations(d.annotations), ref(d.tpe)))
      case d: Def =>
        arr(Seq(
          str("Def"), str(d.name), access(d.access), modifiers(d.modifiers), annotations(d.annotations),
          typeParameters(d.typeParameters), arr(d.valueParameters.map(parameterList)), ref(d.returnType),
        ))
      case d: TypeAlias =>
        arr(Seq(str("TypeAlias"), str(d.name), access(d.access), modifiers(d.modifiers), annotations(d.annotations), typeParameters(d.typeParameters), ref(d.tpe)))
      case d: TypeDeclaration =>
        arr(Seq(
          str("TypeDeclaration"), str(d.name), access(d.access), modifiers(d.modifiers), annotations(d.annotations),
          typeParameters(d.typeParameters), ref(d.lowerBound), ref(d.upperBound),
        ))
      case d: ClassLikeDef =>
        arr(Seq(str("ClassLikeDef"), str(d.name), access(d.access), modifiers(d.modifiers), annotations(d.annotations), typeParameters(d.typeParameters), str(d.definitionType.name)))
      case other => throw new Malformed(s"no node for ${other.getClass.getName}")
    }

    private def path(p: Path): String = arr(p.components.map {
      case id: Id => arr(Seq(str("Id"), str(id.id)))
      case _: This => arr(Seq(str("This")))
      case s: Super => arr(Seq(str("Super"), path(s.qualifier)))
    })

    private def access(a: Access): String = a match {
      case _: Public => str("Public")
      case p: Private => arr(Seq(str("Private"), qualifier(p.qualifier)))
      case p: Protected => arr(Seq(str("Protected"), qualifier(p.qualifier)))
    }

    private def qualifier(q: Qualifier): String = q match {
      case _: Unqualified => str("Unqualified")
      case _: ThisQualifier => str("This")
      case id: IdQualifier => arr(Seq(str("Id"), str(id.value)))
    }

    private def modifiers(m: Modifiers): String = (m.raw & 0xff).toString

    private def annotations(as: Array[Annotation]): String =
      arr(as.map(a => arr(Seq(ref(a.base), arr(a.arguments.map(x => arr(Seq(str(x.name), str(x.value)))))))))

    private def typeParameters(ts: Array[TypeParameter]): String =
      arr(ts.map(t => arr(Seq(str(t.id), annotations(t.annotations), typeParameters(t.typeParameters), str(t.variance.name), ref(t.lowerBound), ref(t.upperBound)))))

    private def parameterList(l: ParameterList): String =
      arr(Seq(arr(l.parameters.map(p => arr(Seq(str(p.name), ref(p.tpe), p.hasDefault.toString, str(p.modifier.name))))), l.isImplicit.toString))
  }

  private def lazily[T](value: => T): Lazy[T] = SafeLazy.apply(new java.util.function.Supplier[T] { def get(): T = value })
}
