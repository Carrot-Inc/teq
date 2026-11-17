# scala3 tests/run through teq

1824 tests

| class | tests |
|---|---|
| pass | 1040 |
| wrong-output | 42 |
| run-error | 70 |
| run-timeout | 0 |
| compile-crash | 0 |
| compile-timeout | 0 |
| compile-error | 672 |

## Compile errors by bucket

| bucket | topic | tests |
|---|---|---|
| gap | typing | 103 |
| gap | standard library | 48 |
| gap | syntax | 34 |
| gap | other | 34 |
| gap | infix call with a block or tuple argument | 28 |
| gap | polymorphic function types | 7 |
| gap | named tuples and named patterns | 6 |
| gap | Float, Byte and Short | 2 |
| gap | local and nested classes | 1 |
| left out on purpose | inline/macro/compiletime | 133 |
| left out on purpose | Java | 102 |
| left out on purpose | reflection (getClass/classOf/ClassTag/deriving) | 65 |
| left out on purpose | Scala 2 syntax (postfix, `f _`, XML, procedure syntax) | 19 |
| left out on purpose | secondary constructors, value classes, Enumeration, self types | 19 |
| left out on purpose | match / dependent / structural types | 17 |
| left out on purpose | Scala 2 implicits | 7 |
| left out on purpose | custom extractors (unapply) | 7 |
| left out on purpose | exceptions (throw/try) | 7 |
| left out on purpose | null | 5 |
| left out on purpose | context functions | 4 |
| left out on purpose | return | 2 |
| left out on purpose | overloading a value | 1 |
| planned | tuples and functions of any arity | 13 |
| planned | anonymous classes | 8 |

## Most frequent gap messages

- 10 × `this is not a case class or enum case` (1938-2: `case ProdNonEmpty(0, _) => ()`)
- 9 × `expected an identifier, found 'super'` (NestedClasses: `override def f = AAA1.super.f;`)
- 9 × `mac is not an extractor: it has no unapply or unapplySeq for an interpolated pattern` (i8577a: `val mac"$x" = 1`)
- 8 × `type ObjectOutputStream not found` (i4446: `val out = new ObjectOutputStream(buffer)`)
- 5 × `boundary is not a member of the package` (break-opt: `import scala.util.boundary, boundary.break`)
- 4 × `a sequence wildcard can only end a sequence pattern` (i11008b: `case Foo(x, xs*) => println(s"Many $x characters $xs")`)
- 4 × `expected ':', found 'val'` (i15840: `class NatOps[N <: Nat](tracked val n: N):`)
- 4 × `value getDeclaredFields is not a member of Class[_]` (capturing: `println(obj.getClass.getDeclaredFields.map(_.toString).sorted.toList.mkString("\`)
- 3 × `augmentString is not a member of the package` (CollectionTests: `import Predef.{augmentString as _, wrapString as _, *}`)
- 3 × `value stringOf is not a member of ScalaRunTime` (enum-values: `extension [A](t: A) def show = runtime.ScalaRunTime.stringOf(t)`)
- 3 × `missing parameter type` (fragables-extension: `x => List(IntFrag(x))`)
- 3 × `expected ']', found '='` (hmap: `TCons(HEntry[K = "name"]("foo"),`)
- 2 × `cannot resolve import: generic not found` (IArrayOps-IsSeq: `import scala.collection.generic.{IsIterable, IsSeq}`)
- 2 × `cannot resolve import: Conversion is not an object or a package` (Parser: `import Conversion.into`)
- 2 × `UnrolledBuffer is not a member of the package` (UnrolledBuffer: `import collection.mutable.UnrolledBuffer`)
- 2 × `expected ',' or ')', found identifier` (collections: `s = s ++ (List.range(0, iters) map (2*))`)
- 2 × `expected ')', found '@'` (enums-serialization-compat: `val Seq(Red @ _, Green @ _, Blue @ _, Indigo @ _) = (1 to 4).map(_ => in.readObj`)
- 2 × `this type cannot be tested at runtime: Matcher[A]` (i13433: `case res: Matcher[A] => Some(res)`)
- 2 × `value gc is not a member of System` (i14198: `System.gc()`)
- 2 × `ambiguous given instances for Foo: foo, foo` (i18183: `assert(foo eq implicitly[Foo])`)
- 2 × `Cannot rewrite recursive call: it is not in tail position` (i20145: `def foo(i: Int): Int = {`)
- 2 × `several entry points: Test.main, Test.main; choose one with --main` (i3339: `object Test extends App {`)
- 2 × `type Product3 not found` (i9068: `case class MyClass(v1: Int, v2: Int, v3: Int, v4: Int) extends Product3[Int, Int`)
- 2 × `missing argument for parameter items` (implicitMatch: `case ord: Ordering[T] => new TreeSet[T]`)
- 2 × `expected end of statement, found ','` (inline-numeric: `given BigDecimalIsFractional: BigDecimalIsConflicted, Fractional[BigDecimal]:`)
- 2 × `type Concat not found` (named-tuples: `type CombinedInfo = NamedTuple.Concat[Person, AddressInfo]`)
- 2 × `cannot resolve import: NonLocalReturns not found` (nonlocal-return: `import scala.util.control.NonLocalReturns.*`)
- 1 × `type a not found` (16405: `case x: ((a, b) => R) =>`)
- 1 × `type mismatch: found "foo", required Extract[M]` (9416: `f(new Ext, "foo")`)
- 1 × `value > is not a member of Int` (ConfManagement: `conf.rankings.takeWhile(conf.score(_) > 80).toSet`)
- 1 × `type mismatch: found Seq[Int], required IArray[Int]` (IArrayOps: `println((iarr1 ++ iarr2.toSeq).toSeq)`)
- 1 × `value convertIArrayToFactory is not a member of IArray` (IArrayOps-Factory: `val explicitConversion: Factory[Int, IArray[Int]] = IArray.convertIArrayToFactor`)
- 1 × `no given instance of type BuildFrom[ArraySeq[Long], Long, IArray[Long]] was found for parameter bf` (IArrayOps-experimental-BuildFrom: `ns.lazyZip(ns.tail).map((a, b) => b - a)`)
- 1 × `no given instance of type Ordering[Iterable[Int]] was found for parameter ord` (OrderingTest: `testAll[Iterable[Int]](List(1), List(1, 2));`)
- 1 × `type mismatch: found Int => Int, required Int` (PrivateAnd: `def callsFoo2(other: B & A): Int = other.foo`)
- 1 × `expected '=>', found '['` (Typeable: `case Typeable.instanceOf[T](x) => "is a"`)
- 1 × `object creation impossible, since given def z[T](using x$1: T): Seq[T] in trait T is not defined` (abstract-givens: `object Test extends T, App:`)
- 1 × `ArrayCharSequence is not a member of the package` (array-charSeq: `import runtime.ArrayCharSequence`)
- 1 × `value apply is not a member of Array[?] & T` (array-erasure: `assert(x(0) == 2)`)
- 1 × `value length is not a member of Q` (array-existential-bound: `def f0(x: Q) = x.length`)

## wrong-output (42)

Course-2002-01, Course-2002-02, Course-2002-04, Course-2002-08, Course-2002-09, Course-2002-10, assert-stack, builder, classtags_contextbound, exceptions-nest, exports, final-fields, i11419, i15725, i2964, i2964b, i2964c, i2964d, i2964e, i3000b, i4947, i4947a2, i4947b, i4947c, i9939, instanceof-serializable, interpolation-opt, interpolationArgs, lambda-null, lazy-val-pattern, range-unit, string-switch, t1074, t1423, t2755, t4859, t5866, t6632, t6827, tuples1b, typetest, virtpatmat_switch

## run-error (70)

deadlock, enum-values-order, equality, export-in-extension, fully-abstract-interface, fully-abstract-nat-2, future-flatmap-exec-count, hashhash, hk-alias-unification, i11174, i11174local, i11938, i12328a, i1240, i12976, i13181, i13332shapeless-b, i14020, i14340, i14623, i15374, i16806, i19224, i1990b, i22649, i2275, i24056, i24204, i26800-trait-init-binary-compat, i2738, i3006, i3006b, i3340, i4659b, i502, i7843, i7960, i9155, lazy-exprs, lift-and-unlift, llift, numbereq, optimizer-array-load, reducable, string-extractor, t2378, t3088, t3235-minimal, t3273, t4024, t4742, t5680, t5857, t6064, t6090, t6154, t6333, t6406-regextract, t6534, t7120b, t7336, t7396, t7912, t8199, t8601b, t8601d, traitNoInit, transparentProtected, uninitialized-field-values, unroll-multiple

## run with the flags their options give (103)

- companion-loading: `--language experimental.erasedDefinitions`
- erased-18: `--language experimental.erasedDefinitions`
- erased-19: `--language experimental.erasedDefinitions`
- erased-20: `--language experimental.erasedDefinitions`
- erased-21: `--language experimental.erasedDefinitions`
- erased-25: `--language experimental.erasedDefinitions`
- erased-26: `--language experimental.erasedDefinitions`
- erased-7: `--language experimental.erasedDefinitions`
- erased-lambdas: `--language experimental.erasedDefinitions`
- erased-poly-ref: `--language experimental.erasedDefinitions`
- errorhandling: `--language experimental.captureChecking,experimental.separationChecking`
- i11996: `--language experimental.erasedDefinitions`
- i15840: `--source future --language experimental.modularity`
- i23305: `--language experimental.erasedDefinitions`
- i25725: `--language experimental.erasedDefinitions`
- i3920: `--source future --language experimental.modularity`
- inline-trait-body-lazy-val: `--language experimental.inlineTraits`
- inline-trait-body-override-def: `--language experimental.inlineTraits`
- inline-trait-body-override-val: `--language experimental.inlineTraits`
- inline-trait-body-statements: `--language experimental.inlineTraits`
- inline-trait-body-var: `--language experimental.inlineTraits`
- inline-trait-child-and-grandchild-override: `--language experimental.inlineTraits`
- inline-trait-clash-method-method: `--language experimental.inlineTraits`
- inline-trait-clash-method-val-param: `--language experimental.inlineTraits`
- inline-trait-clash-param-method: `--language experimental.inlineTraits`
- inline-trait-clash-param-param: `--language experimental.inlineTraits`
- inline-trait-clash-param-val-param: `--language experimental.inlineTraits`
- inline-trait-clash-param-var-param: `--language experimental.inlineTraits`
- inline-trait-clash-val-param-val-param: `--language experimental.inlineTraits`
- inline-trait-clashing-parent-params: `--language experimental.inlineTraits`
- inline-trait-enclosing-super-call: `--language experimental.inlineTraits`
- inline-trait-enclosing-supercall-in-inline-trait: `--language experimental.inlineTraits`
- inline-trait-indirect-with-params: `--language experimental.inlineTraits`
- inline-trait-inheritance-diamond-simple-trait: `--language experimental.inlineTraits`
- inline-trait-inheritance-inline-ancestors: `--language experimental.inlineTraits`
- inline-trait-inheritance-inline-grandparent: `--language experimental.inlineTraits`
- inline-trait-maximum-name-clash: `--language experimental.inlineTraits`
- inline-trait-nested: `--language experimental.inlineTraits`
- inline-trait-non-inline-blocks-inlining: `--language experimental.inlineTraits`
- inline-trait-override-in-parent-triangle: `--language experimental.inlineTraits`
- inline-trait-param-shadows-parent-indirect: `--language experimental.inlineTraits`
- inline-trait-signature-parameters-val-block: `--language experimental.inlineTraits`
- inline-trait-signature-side-effects-1: `--language experimental.inlineTraits`
- inline-trait-signature-side-effects-2: `--language experimental.inlineTraits`
- inline-trait-specialized-desugar: `--language experimental.specializedTraits`
- inline-trait-super-call: `--language experimental.inlineTraits`
- inline-trait-super-chain: `--language experimental.inlineTraits`
- inline-trait-varargs: `--language experimental.inlineTraits`
- lambda-serialization-others: `--language experimental.erasedDefinitions`
- named-tuple-ops: `--source future`
- named-tuples-strictEquality: `--language strictEquality`
- runtimeChecked: `--source future`
- runtimeChecked-2: `--source future`
- specialized-trait-all-general-raw: `--language experimental.specializedTraits`
- specialized-trait-argument-specializes-current-trait: `--language experimental.specializedTraits`
- specialized-trait-array-of-specialized-trait-classtag: `--language experimental.specializedTraits`
- specialized-trait-as-parameter: `--language experimental.specializedTraits`
- specialized-trait-as-return-type: `--language experimental.specializedTraits`
- specialized-trait-as-val: `--language experimental.specializedTraits`
- specialized-trait-body-macro-uses-type-param: `--language experimental.specializedTraits`
- specialized-trait-call-method-in-other-file: `--language experimental.specializedTraits`
- specialized-trait-check-bridge-exists: `--language experimental.specializedTraits`
- specialized-trait-check-bridge-exists-hidden-by-inline: `--language experimental.specializedTraits`
- specialized-trait-check-specialized-method-called: `--language experimental.specializedTraits`
- specialized-trait-class-extends-specialized-trait-check-specialized: `--language experimental.specializedTraits`
- specialized-trait-collections-example: `--language experimental.specializedTraits`
- specialized-trait-diamond: `--language experimental.specializedTraits`
- specialized-trait-erasure: `--language experimental.specializedTraits`
- specialized-trait-erasure-impl-classes: `--language experimental.specializedTraits`
- specialized-trait-inline-specialization-macro: `--language experimental.specializedTraits`
- specialized-trait-inline-specialized-instance: `--language experimental.specializedTraits`
- specialized-trait-inline-specialized-instance-hidden: `--language experimental.specializedTraits`
- specialized-trait-inline-specialized-instance-with-specialization: `--language experimental.specializedTraits`
- specialized-trait-inline-trait-ordering-edge-case: `--language experimental.specializedTraits`
- specialized-trait-inlining-loop: `--language experimental.specializedTraits`
- specialized-trait-list-of-specialized-trait: `--language experimental.specializedTraits`
- specialized-trait-list-zip-example: `--language experimental.specializedTraits`
- specialized-trait-manual-class-extend-not-top-of-hierarchy: `--language experimental.specializedTraits`
- specialized-trait-masked-inline-specialization-via-inlining: `--language experimental.specializedTraits`
- specialized-trait-masked-inline-specialized: `--language experimental.specializedTraits`
- specialized-trait-maths: `--language experimental.specializedTraits`
- specialized-trait-multiple-files: `--language experimental.specializedTraits`
- specialized-trait-multiple-package-same-file: `--language experimental.specializedTraits`
- specialized-trait-multiple-package-same-file-nested: `--language experimental.specializedTraits`
- specialized-trait-multiple-parameter-packs: `--language experimental.specializedTraits`
- specialized-trait-multiple-parameter-packs-method: `--language experimental.specializedTraits`
- specialized-trait-one-parent-is-inline: `--language experimental.specializedTraits`
- specialized-trait-package: `--language experimental.specializedTraits`
- specialized-trait-package-contains-object: `--language experimental.specializedTraits`
- specialized-trait-package-contains-object-multi-file: `--language experimental.specializedTraits`
- specialized-trait-pathological-context-bounds: `--language experimental.specializedTraits`
- specialized-trait-requires-inline-trait-inlining: `--language experimental.specializedTraits`
- specialized-trait-scoped-inside-object: `--language experimental.specializedTraits`
- specialized-trait-scoped-inside-object-2: `--language experimental.specializedTraits`
- specialized-trait-scoped-inside-object-3: `--language experimental.specializedTraits`
- specialized-trait-self-inline: `--language experimental.specializedTraits`
- specialized-trait-subtyping-once-specialized: `--language experimental.specializedTraits`
- specialized-trait-super-chain: `--language experimental.specializedTraits`
- specialized-trait-varargs: `--language experimental.specializedTraits`
- specialized-trait-variance: `--language experimental.specializedTraits`
- specialized-trait-vector-dot-product: `--language experimental.specializedTraits`
- specialized-trait-vector-zip: `--language experimental.specializedTraits`
- trait-extends-inline-trait-uses: `--language experimental.inlineTraits`

## pass (1040)

1938, 19384, 19384-min, 25333, 26071, 26957, 3179, Course-2002-03, Course-2002-05, Course-2002-06, Course-2002-07, Course-2002-13, HLists, HLists-nonvariant, HelloWorld, Lazies1, Lazies2, LazyLists, LazyValsLongs, Meter, MultiArr, Pouring, Pouring2, QueueTest, ReverseSeqView, Signals1, Signals2, StackMap, Tuple-fromArray, Tuple-fromProduct, absoverride, adding-growing-set, array-addition, arrayclone-new, arraycopy, arrays, arrayview, arybufgrow, automatic-tupling-of-function-parameters-implicit-conversion, automatic-tupling-of-function-parameters-spec, better-fors, better-fors-map-elim, better-fors-map-inlined, bigDecimalCache, bigDecimalTest, bind-tuple-pattern, blame_eye_triple_eee-double, blame_eye_triple_eee-float, block-field, boehm-berarducci, boolexprs, buffer-slice, byNameVarargs, byname-implicits-17, byname-implicits-28, byname-implicits-30, byname-implicits-4, byname-implicits-5, byname-implicits-6, byname-param, byname-varargs, case-class-23, case-class-toString, caseClassHash, case_class_equals_fields_sort, classTags, classof-object, cochis-example, collection-stacks, collections-toSelf, collective-extensions, colltest, complicatedmatch, concat-two-strings, constValueTuple, constant-optimization, constructors, contextbounds-for-poly-functions, contrarivant, contrib674, coop-equality, correct-bind, curried-using-threading, dead-code-elimination, defaultGetters, defunctionalized, delambdafy-dependent-on-param-subst, delambdafy-dependent-on-param-subst-2, delambdafy-nested-by-name, delambdafy-two-lambdas, delayedInit, derived-specificity, deriving, deriving-constructor-order, deriving-from-product-typed, distinct, drop-no-effects, duplicate-meth, duration-coarsest, emptypf, enum-Color, enum-List1, enum-List2, enum-List2a, enum-Option, enum-Option1, enum-Tree, enum-approx, enum-constrs, enum-in-trait, enum-ordinal-java, enums-precise, enums-thunk, eq-xmethod, erased-inline-product-3, eta-extension, evaluation-order, exc, exc1, exc2, exceptions, exoticnames, experimentalRun, export-anyval, extension-methods, extension-override, extension-specificity, extension-specificity2, extmethod-overload, final-var, finally, flat-flat-flat, for-desugar-strawman, fors, forvaleq, fully-abstract-nat-1, fully-abstract-nat-3, function-arity, functionXXL, gadts, generic, genericTupleMembers, genericValueClass, gestalt-optional-inline, getClassTest-valueClass, given-eta, given-triangle, given-var, groupby, hashCodeDistribution, hashsetremove, hello, i10016, i10047, i10062, i10068a, i10068b, i10068c, i10082, i10178, i10285, i10527, i10724, i10857, i10866, i10905, i1099, i11008, i11043, i11045, i11050, i1140, i1144, i11542, i11542a, i11563, i11676, i11706, i11793, i11914, i11914a, i11961, i11966, i12052, i12160, i12170, i12328b, i12597, i1263, i12729, i12759, i12759b, i12796, i12828a, i12828b, i12829, i1284, i12919, i12919a, i12919b, i13087a, i13096, i13131, i13146, i13146a, i13183, i13215, i13216, i13228, i13252a, i13252b, i13304, i13334, i13358, i13490.min, i13630, i13747, i1386, i13862, i1392a, i1392b, i1392c, i13968, i13986, i13987, i14127, i14164, i14215, i1423, i1441, i14432, i14432a, i14432b, i14432c, i14540-priorRun, i14540-sameRun, i14582, i14587, i14587.min, i14587.opaques, i1463, i14667, i14675, i14693, i14705, i14964, i14964b, i14970, i14983, i1503, i15101, i15190, i15302b, i15315, i15317, i1533, i15399, i1543, i15618, i1569, i15723, i1573, i15913, i15943, i15988a, i15988b, i16065, i16092, i16108, i16213, i16252, i16390, i16728, i16785, i16879, i17021, i17021.defs, i17317, i17317-b, i1732, i1732_1, i17332, i1748, i17549, i17555, i17584, i17584a, i1773, i17761, i1779, i17930, i1820, i1820b, i1856, i18612-a, i18612-b, i18612-c, i18612-d, i18638, i18884, i1915, i19178, i19396, i19587, i1960, i19629, i19641, i19711, i19724, i1991, i2004, i2004b, i20095, i2020, i20225, i20284, i20395, i2077, i209, i2146, i2147, i2156, i2163, i22345, i22498, i2266, i22888, i22900a, i2314, i23179, i23245b, i23245c, i23245d, i23279, i2337, i2337b, i23409, i23409b, i23444, i23477, i2360, i23693, i23776, i23875, i23901, i2396, i2396b, i2396c, i24201a, i24201b, i24357, i24420-transparent-inline-local-ref, i2456, i24573, i24673, i24826, i24877, i24926, i24998, i25000, i25000b, i25077, i2508, i25089, i25291, i2567, i25943, i26176, i26176-with-implicits, i26349, i2795, i2808, i2895a, i2916, i2939, i3018, i3189, i3200c, i3248, i3248b, i3248c, i3396, i3548, i3624, i3930, i4037, i4073b, i4073c, i4177, i4205, i4241, i4364b, i4410, i4430, i4451, i4496a, i4496b, i4557a, i4558, i4559, i4563, i4600b, i4754, i4935, i4935b, i4961, i505, i5067, i5067b, i5224, i5257, i5260, i5340, i5350, i5350b, i5350c, i5350d, i5386, i5441, i5606, i5791, i5823, i5924, i5924b, i5924c, i5976, i6109, i6341, i6490, i659, i6664, i6664b, i6677, i6710, i6716, i6816, i6858, i689, i6891, i6902, i6987, i6987b, i6996, i7031, i7049, i7086, i7110, i7281, i7287, i7359, i7375, i7410, i7424, i744, i756, i7613, i763, i7630, i764, i7675, i7677, i768, i7783, i7798, i789, i7926, i7991, i7991, i8035, i8058, i806, i8064, i8096, i8314, i8396, i8398, i8530, i8662, i8726, i8903, i8931, i9011, i9056, i9132, i9161, i9404, i9439, i9473, i9507, i9928, implicit-disambiguation, implicit-functors, implicit-functors2, implicit-specifity-2, implicitFunctionXXL, implicitclasses, implicits_poly, implied-divergence, implied-for, imports, indent, indexedSeq, indexedSeq-apply, infiniteloop, infix, inline-override, inline-override-num, inline-param-semantics, inliner-infer, innerClass, innerObject, instances, instances-anonymous, invocation-receivers, invocationReceivers1, invocationReceivers2, irrefutable, isInstanceOf-eval, iterator-concat, iterator-iterate-lazy, iterator3444, iterators, lambda-sam-bridge, lambda-unit, lazy-implicit-lists, lazy-implicit-nums, lazy-override-run, lazy-traits, lazy-val-pattern-2, lazy-val-pattern-3, lazyVals, lazyVals_c3.0.0, lazyVals_c3.1.0, liftedTry, list-apply-eval, list_map, lists-run, literals, longmap, main-functions, mapConserve, mapValues, matchable, matcharraytail, matchbytes, matchemptyarray, matchintasany, matchnull, matchonseq, matchonstream, memoTest, missingparams, mixin-final-def-object-lucre, mixin-overrides, mixin-primitive-on-generic-1, mixin-primitive-on-generic-2, mixin-primitive-on-generic-3, mixin-primitive-on-generic-4, mixin-primitive-on-generic-5, mixins, mixins1, multi-apply, named-patmatch, named-tuples-strictEquality, nats, nested-object-implements-val, nestedEq, no-init-enclosing-static-objects, noProtectedSuper, non-jvm-sam-non-apply, nonLocalReturns, nonlocalreturn, nothingTypeDce, nothingTypeNoFramesNoDce, nothingTypeNoOpt, null-and-intersect, null-hash, nullAsInstanceOf, nullInstanceEval, numeric-range, opaque-immutable-array-xm, opaque-inline2, option-fold, ordered, outer-accessors, outerPatternMatch, overload_directly_applicable, overloading-specifity, overloads, partialFunctions, patch-boundary, patmat, patmat-bind-typed, patmat-finally, patmat-option-named, phantomValueClass, priorityQueue, prod-mirror-inacessible-ctor, promotion, protectedSuper, pure-args-byname-noinline, puzzle, puzzler54, quote-MacroOverride, quoted-sematics-1, rainwater, rawstrings, recursive-with-option, reduce-projections, retclosure, retsynch, returning, returns, richWrapperEquals, rooted_stringcontext, run-bug4840, runtime, runtime-richChar, runtimeChecked, runtimeChecked-2, sammy_repeated, scala_enum_testing, scan, selectable-new, simpleClass, singletons, sip23-valueof, slice-strings, slices, sort, sorted-sets-ok, spec-self, stable-enum-hashcodes, static, static-module-method, statics, sticky-extmethod, stream-stack-overflow-filter-map, streamWithFilter, string-switch-defaults-null, stringbuilder-drop, structural-contextual, summonAll, supercalls-traits, supercalls-traits-targetname, switches, synchronized-interface, t0005, t0017, t0042, t0048, t0325, t0607, t0631, t0668, t0807, t0883, t0936, t10032, t1005, t10170, t1044, t1048, t10594, t107, t1192, t1220, t1300, t13033, t1309, t1333, t1335, t13418, t1360, t1373, t1381, t1434, t1466, t1500b, t1500c, t1524, t153, t1535, t1591, t1672, t1697, t1718, t1747, t1909b, t1909c, t1939, t1987, t1987b, t1994, t2005, t2027, t2029, t2087-and-2400, t2127, t2147, t216, t2175, t2176, t2177, t2255, t2316_run, t2333, t2418, t2446, t2488, t2526, t2552, t2594_tcpoly, t261, t266, t2754, t2788, t2818, t2857, t2867, t2876, t2958, t298, t3004, t3026, t3038, t3038b, t3038c, t3048, t3097, t3112, t3199b, t3232, t3241, t3242b, t3269, t3326, t3346a, t3346d, t3353, t3395, t3397, t3452, t3452c, t3452f, t3487, t3488, t3496, t3508, t3509, t3516, t3529, t3563, t3603, t3619, t363, t3645, t3651, t3670, t3699, t3702, t3714, t3726, t3760, t3761-overload-byname, t3763, t3832, t3835, t3855, t3877, t3895, t3923, t3935, t3980, t3994, t4013, t4013b, t4013c, t405, t4054, t4062, t408, t4122, t4148, t4190, t4288, t429, t4297, t4351, t4398, t4400, t4415, t4482, t4537, t4558, t4565_1, t4577, t4582, t4592, t4601, t4660, t4697, t4709, t4750, t4753, t4770, t4777, t4809, t4827, t4827b, t4835, t4897, t493, t4935, t4954, t498, t4996, t5009, t5037, t5053, t5105, t5158, t5162, t5171, t5201, t5284b, t5284c, t5300, t5328, t5387, t5394, t5407, t5530, t5532, t5552, t5568, t5604, t5608, t5610a, t5614, t5629, t5629b, t5648, t5665, t5688, t5729, t5830, t5856, t5923c, t594, t5971, t601, t6011b, t6011c, t603, t6070, t6077_patmat_cse_irrefutable, t6089, t6104, t6111, t6126, t6188, t6206, t6260, t627, t6272, t629, t6290, t6337a, t6370, t6385, t6443-by-name, t6443-varargs, t6476, t6500, t6506, t6541, t6559, t6574b, t6584, t6611, t6611b, t6637, t6644, t6646, t6666a, t6673, t6888, t6928-run, t6957, t6968, t7019, t704, t7120, t7126, t7171, t7181, t7215, t7223, t7231, t7242, t7249, t7278, t7300, t7341, t7406, t7407, t7407b, t7436, t7445, t7475b, t7571, t7584, t7584b, t7763, t7859, t7899, t7985, t7985b, t8002, t8010, t8015-ffc, t8017, t8100, t8133, t8133b, t8197, t8197b, t8233, t8233-bcode, t8245, t8280, t8395, t8428, t8570, t8570a, t8601, t8607, t8610, t8611a, t8611c, t8738, t8845, t8888, t889, t8893, t8893b, t8933b, t8933c, t920, t920_1, t9516, t978, takeAndDrop, targetName, targetName-separate, targetName1, tcpoly_monads, tcpoly_overriding, test-implicits, toList, toplevel-defs, toplevel-implicits, trailingCommas, traitInit, traitParamInit, traitParams, traitValBridge, traitValOverriddenByParamAccessor, traits-initialization, transparent-foreach, transparent-implicits, transparent-object, transparentAccess, transparentAssign, transparentByName, transparentForeach, transparentPower, transparentPrivates, transpose, triple-quoted-expr, try, try-2, tryPatternMatch, tuple-accessors, tuple-concat, tuple-cons, tuple-cons-2, tuple-deconstruct-wildcard-sideeffect, tuple-erased, tuple-for-comprehension, tuple-match, tuple-patmat, tuple-patmat-extract, tuple-patmat-size, tuple-patterns, tuple-product, tuple-split, tuple-tail, tuple-typetests, tuple-underscore-syntax, tuple-zip, tuples-empty, tuples1a, typable, type-test-binding, type-test-nat, typeclass-derivation-doc-example, typelevel-overrides, unapplyArray, unboxingBug, unchecked-patterns, unittest_iterator, unreachable, unroll-inferredFinal, value-class-partial-func-depmet, valueclasses-constr, valueclasses-nested-object, valueclasses-pavlov, vc-equals, verify-ctor, view-headoption, virtpatmat_alts, virtpatmat_apply, virtpatmat_casting, virtpatmat_literal, virtpatmat_npe, virtpatmat_opt_sharing, virtpatmat_partial_backquoted, virtpatmat_stringinterp, virtpatmat_tailcalls_verifyerror, virtpatmat_try, virtpatmat_typed, virtpatmat_unapply, virtpatmat_unapplyseq, virtpatmat_valdef, weakconform, whitebox-inline, wildcard-vals, withIndex
