pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Erdos_PqT_Spark
  with SPARK_Mode => On
is

   function Verify_Sunflower_Origo_Kernel
     (Petal_A_Mask    : Unsigned_64;
      Petal_B_Mask    : Unsigned_64;
      Origo_Core_Mask : Unsigned_64) return Boolean
   is
   begin
      return Origo_Core_Mask /= 0
        and then (Petal_A_Mask and Petal_B_Mask) = Origo_Core_Mask;
   end Verify_Sunflower_Origo_Kernel;

end Erdos_PqT_Spark;
