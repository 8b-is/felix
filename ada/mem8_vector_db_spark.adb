pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations

package body Mem8_Vector_Db_Spark is

   function Compute_Rrf_Numerator_Same_Rank
     (Rank : Rank_Range) return Unsigned_32
   is
      pragma Unreferenced (Rank);
   begin
      return 3 + 3 + 2;
   end Compute_Rrf_Numerator_Same_Rank;

   function Compute_Rrf_Denominator_Same_Rank
     (Rank : Rank_Range) return Unsigned_32
   is
   begin
      return Rrf_K + Rank;
   end Compute_Rrf_Denominator_Same_Rank;

   procedure Verify_Mem8_Vector_Wave_Db_24_1
     (Cert : out Mem8_Vector_Db_Cert)
   is
      Comp_Ok  : constant Boolean :=
        (Raw_Fp16_Embedding_Bytes = Compressed_Code32_Bytes * Vector_Compression_P)
        and then (Vector_Compression_Q = 1);
      Chunk_Ok : constant Boolean :=
        (Overlap_Tokens + Retention_Stride_Tokens = Chunk_Tokens)
        and then (4 * Overlap_Tokens = Chunk_Tokens)
        and then (4 * Retention_Stride_Tokens = 3 * Chunk_Tokens);
      Top1_Num : constant Unsigned_32 := Compute_Rrf_Numerator_Same_Rank (1);
      Top1_Den : constant Unsigned_32 := Compute_Rrf_Denominator_Same_Rank (1);
      Top2_Num : constant Unsigned_32 := Compute_Rrf_Numerator_Same_Rank (2);
      Top2_Den : constant Unsigned_32 := Compute_Rrf_Denominator_Same_Rank (2);
      Mono_Ok  : constant Boolean :=
        (Top1_Num * Top2_Den > Top2_Num * Top1_Den);
   begin
      Cert :=
        (Compression_Exact_24_1 => Comp_Ok,
         Chunk_Lovasz_3_4_Valid => Chunk_Ok,
         Rrf_Top1_Num           => Top1_Num,
         Rrf_Top1_Den           => Top1_Den,
         Rrf_Monotone_Valid     => Mono_Ok,
         All_Verified           => Comp_Ok and then Chunk_Ok and then Mono_Ok);
   end Verify_Mem8_Vector_Wave_Db_24_1;

end Mem8_Vector_Db_Spark;
