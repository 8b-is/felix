pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Exception_Propagation);

with Interfaces;           use Interfaces;
with Embedded_Quant_Peter; use Embedded_Quant_Peter;
with Felix_Nand_TCB;       use Felix_Nand_TCB;

package Omni_Genesis_Seed_Spark
  with Pure, SPARK_Mode => On
is
   type M8_Magic_Bytes is array (1 .. 8) of Unsigned_8;
   type Triadic_Seed_Poles is record
      AllFather : Balanced_Trit := 1;
      Sun       : Balanced_Trit := 0;
      Dharma    : Balanced_Trit := -1;
   end record;

   Expected_Magic : constant M8_Magic_Bytes :=
     (16#4D#, 16#45#, 16#4D#, 16#38#, 16#03#, 16#09#, 16#1B#, 16#F3#);

   CAP_ROOT_SEAL : constant Capability_Mask := 16#8000_0000#;

   function Verify_M8_Container_Header
     (Magic                 : M8_Magic_Bytes;
      Poles                 : Triadic_Seed_Poles;
      Declared_Effects      : Capability_Mask;
      Spectral_Radius_Micro : Unsigned_32;
      Seal_Matches          : Boolean) return Boolean is
     (Magic = Expected_Magic
      and then (Integer (Poles.AllFather) + Integer (Poles.Sun) + Integer (Poles.Dharma) = 0)
      and then (Declared_Effects and CAP_ROOT_SEAL) = 0
      and then Spectral_Radius_Micro <= 880_000
      and then Seal_Matches)
     with Inline;

end Omni_Genesis_Seed_Spark;
