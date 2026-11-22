public @interface Ann { int x() default 1; String s(); String[] arr() default {}; Inner in() default @Inner(v = 9); }
