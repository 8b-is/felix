pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations

with Interfaces; use Interfaces;

package Mem8_Vector_Db_Spark is

   --  384-dim FP16 raw embedding bytes (384 * 2 = 768 bytes)
   Raw_Fp16_Embedding_Bytes : constant Unsigned_32 := 768;

   --  Compressed E8-Gosset (16B) + SPQT Base-243 (16B) = 32 bytes
   Compressed_Code32_Bytes  : constant Unsigned_32 := 32;

   --  Canonical reduced vector index compression ratio p/q = 768 / 32 = 24 / 1
   Vector_Compression_P     : constant Unsigned_32 := 24;
   Vector_Compression_Q     : constant Unsigned_32 := 1;

   --  Sliding-window chunking with LLL Lovasz 3/4 retention (1/4 overlap)
   Chunk_Tokens             : constant Unsigned_32 := 64;
   Overlap_Tokens           : constant Unsigned_32 := 16;
   Retention_Stride_Tokens  : constant Unsigned_32 := 48;

   --  Reciprocal Rank Fusion (RRF) smoothing constant k = 60
   Rrf_K                    : constant Unsigned_32 := 60;

   subtype Rank_Range is Unsigned_32 range 1 .. 10_000;

   type Mem8_Vector_Db_Cert is record
      Compression_Exact_24_1 : Boolean;
      Chunk_Lovasz_3_4_Valid : Boolean;
      Rrf_Top1_Num           : Unsigned_32;
      Rrf_Top1_Den           : Unsigned_32;
      Rrf_Monotone_Valid     : Boolean;
      All_Verified           : Boolean;
   end record;

   function Compute_Rrf_Numerator_Same_Rank
     (Rank : Rank_Range) return Unsigned_32
   with
     Post => Compute_Rrf_Numerator_Same_Rank'Result = 8;

   function Compute_Rrf_Denominator_Same_Rank
     (Rank : Rank_Range) return Unsigned_32
   with
     Post => Compute_Rrf_Denominator_Same_Rank'Result = Rrf_K + Rank
       and then Compute_Rrf_Denominator_Same_Rank'Result >= 61;

   procedure Verify_Mem8_Vector_Wave_Db_24_1
     (Cert : out Mem8_Vector_Db_Cert)
   with
     Post => Cert.Compression_Exact_24_1
       and then Cert.Chunk_Lovasz_3_4_Valid
       and then Cert.Rrf_Top1_Num = 8
       and then Cert.Rrf_Top1_Den = 61
       and then Cert.Rrf_Monotone_Valid
       and then Cert.All_Verified;

end Mem8_Vector_Db_Spark;
