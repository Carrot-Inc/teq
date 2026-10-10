# scala3 tests/run through teq

322 tests

| class | tests |
|---|---|
| pass | 94 |
| wrong-output | 48 |
| run-error | 19 |
| run-timeout | 0 |
| compile-crash | 0 |
| compile-timeout | 0 |
| compile-error | 161 |

## Compile errors by bucket

| bucket | topic | tests |
|---|---|---|
| gap | standard library | 37 |
| gap | typing | 13 |
| gap | other | 9 |
| gap | syntax | 3 |
| gap | named tuples and named patterns | 2 |
| left out on purpose | inline/macro/compiletime | 77 |
| left out on purpose | Java | 6 |
| left out on purpose | extends App | 6 |
| left out on purpose | match / dependent / structural types | 3 |
| left out on purpose | exceptions (throw/try) | 2 |
| left out on purpose | Scala 2 syntax (postfix, `f _`, XML, procedure syntax) | 2 |
| left out on purpose | reflection (getClass/classOf/ClassTag/deriving) | 1 |

## Most frequent gap messages

- 33 × `MacroAnnotation is not a member of the package` (annot-add-global-class: `import scala.annotation.{experimental, MacroAnnotation}`)
- 4 × `value apply is not a member of PolyType` (newClassTraitAndAbstract: `(classType: TypeRepr) => PolyType(List("A", "B"))(`)
- 2 × `value Predef is not a member of scala` (i5533b: `scala.Predef.assert(_result)`)
- 1 × `FromDigits is not a member of the package` (BigFloat: `import scala.util.FromDigits`)
- 1 × `None of the overloaded alternatives of method let in object ValDef with types` (ValDef-let-flags: `(owner: Symbol, terms: List[Term])(body: List[Ref] => Term): Term`)
- 1 × `type SuppressWarnings not found` (annot-java-tree: `val SuppressWarningsSymbol = TypeTree.of[SuppressWarnings].symbol`)
- 1 × `type mismatch: found String, required String & T` (expr-map-3: `val s2: String & T = s`)
- 1 × `no given instance of type ToExpr[(String, List[String], String, String, List[String])] was found for parameter to` (expr-mirror-info: `Expr((mirroredLabel, mirroredElemLabels, mirroredMonoTypeString, mirroredTypeStr`)
- 1 × `expected end of statement, found identifier` (f-interpolator-tests: `end `f interpolator baseline``)
- 1 × `expected ']', found '['` (i16734a: `println(variances[[A, F[B]] =>> F[A]])`)
- 1 × `not found: GivenSelector` (i21225: `val givenSelector: Selector = GivenSelector(None)`)
- 1 × `not found: MethodTypeKind` (i22260: `val tpe = MethodType(MethodTypeKind.Contextual)("x" :: Nil)(_ => TypeRepr.of[Int`)
- 1 × `type mismatch: found S => Int, required S => 1` (i5941: `Iso[S, 1](Function.const($ident))(Function.const(1))`)
- 1 × `type argument Function1 does not have the same kind as its parameter T` (i6518: `val classSym = TypeRepr.of[Function1].classSymbol.get`)
- 1 × `type argument T does not have the same kind as its parameter T` (i8520: `val t = TypeRepr.of[T].typeSymbol.memberTypes.map(x => (x.name, variance(x.flags`)
- 1 × `there is no parameter named clsFlags` (newClassParams: `val cls = Symbol.newClass(`)
- 1 × `too many arguments: expected 5` (newClassParamsExtendsClassParams: `val cls = Symbol.newClass(Symbol.spliceOwner, name, parents = _ => parents.map(_`)
- 1 × `Not a constant` (opaque-inline: `val em = EmailAddress("a@b.c")`)
- 1 × `value asExprOf is not a member of Expr[Tuple]` (quote-toExprOfTuple: `res.asExprOf[(T0, T1)]`)
- 1 × `value newTypeAlias is not a member of Symbol` (refined-apply: `val typeDefSym = Symbol.newTypeAlias(Symbol.spliceOwner, "test1", Flags.EmptyFla`)
- 1 × `not found: augmentString` (reflect-inline: `Expr(augmentString(x.valueOrAbort).stripMargin)`)
- 1 × ``yield` or `do` expected` (simple-interpreter: `for kSchema <- interpretSchema(kSchemaExpr)`)
- 1 × `not a match` (switch-match: `unswitch {`)
- 1 × `type argument List does not have the same kind as its parameter T` (tasty-construct-types: `val x6T = TypeRepr.of[List].appliedTo(List(TypeRepr.of[Int]))`)
- 1 × `value box is not a member of Int` (tasty-extractors-1: `printTree(Int.box(x = 9))`)
- 1 × `Expected statically known StringContext` (tasty-interpolation-1: `println(s2"Hello $w!")`)
- 1 × `Parameter must be a known constant` (tasty-macro-const: `println(natConst(2))`)
- 1 × `type argument a does not have the same kind as its parameter T[_]` (tasty-simplified: `case Wrap[Apply[a]] => F[a]`)

## wrong-output (48)

Xmacro-settings-compileTimeEnv, expr-map-2, flops-rewrite, flops-rewrite-2, flops-rewrite-3, from-type, i12392, i14902, i15968, i18283, i20449, i22395, i5119b, i6765, i6765-b, i6765-c, i7898, i7987, i8514, i8514b, inline-beta-reduce-polyfunction, paramSymss, quote-impure-by-name, quote-inline-function, quote-match-poly-function-1, quote-match-poly-function-1-regression, quote-matching-optimize-1, quote-matching-optimize-2, quote-matching-optimize-3, quote-matching-optimize-4, quote-matching-optimize-5, reflect-sourceCode, self, summonIgnoring, tasty-argument-tree-1, tasty-dealias, tasty-dealiasKeepOpaques, tasty-definitions-1, tasty-definitions-2, tasty-definitions-3, tasty-eval, tasty-extractors-3, tasty-extractors-types, tasty-load-tree-1, tasty-load-tree-2, tasty-original-source, tasty-typeof, type-print

## run-error (19)

TypeRepr-isTupleN, Xmacro-settings-simple, expr-map-1, gestalt-type-toolbox-reflect, i12351, i12352, i17445, i17445-new-default, i4947e, i4947f, i5119, i6253-c, i6270, i6803, i8007, is-in-typer, no-symbol, tasty-positioned, tasty-string-interpolation-reporter-test

## pass (94)

f-interpolator, i10011, i10043, i10464, i10880, i10914a, i10914b, i11161, i11856, i12163, i12188, i12417b, i13033, i13183, i13230, i13947, i17257, i17445-copy, i22616c, i4431-b, i4455, i4492, i4515, i4515b, i4734, i4735, i4803, i4803b, i4803c, i5110, i5188a, i5533, i6201, i6253, i6253-b, i6622, i6679, i6772, i7008, i7025, i7025b, i7048, i7715, i8115, i8115b, i8306, i8530, i8671, i8745, i8745b, i8746b, i8877, i9475, inferred-repeated-result, inline-macro-inner-object, inline-macro-staged-interpreter, inline-option, inline-tuples-2, inline-varargs-1, power-macro, quote-and-splice, quote-change-owner, quote-elide-prefix, quote-force, quote-indexed-map-by-name, quote-matcher-inference, quote-matcher-power, quote-matcher-string-interpolator, quote-matcher-string-interpolator-2, quote-matcher-string-interpolator-3, quote-matcher-symantics-1, quote-matcher-type-bind, quote-sep-comp, quote-sep-comp-2, quote-simple-macro, quote-toExprOfSeq, quote-type-matcher-2, quote-unrolled-foreach, quote-whitebox, quoted-expr-block, summonIgnoring-nonrecursive, tasty-extractors-constants-1, tasty-getfile, tasty-indexed-map, tasty-linenumber, tasty-linenumber-2, tasty-location, tasty-subtyping, tasty-unsafe-let, whitebox-inline-macro, xml-interpolation-3, xml-interpolation-4, xml-interpolation-5, xml-interpolation-6
