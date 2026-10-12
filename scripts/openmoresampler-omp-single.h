#ifndef OPENMORESAMPLER_OMP_SINGLE_H
#define OPENMORESAMPLER_OMP_SINGLE_H

/* MinGW on macOS does not ship a target OpenMP runtime with the Homebrew
 * toolchain. Keep the generated Windows fallback deterministic and single
 * threaded; native builds continue to use the upstream OpenMP runtime. */
#define omp_get_num_threads() 1
#define omp_get_thread_num() 0
#define omp_get_max_threads() 1
static inline void omp_set_num_threads(int threads) { (void)threads; }

#endif
