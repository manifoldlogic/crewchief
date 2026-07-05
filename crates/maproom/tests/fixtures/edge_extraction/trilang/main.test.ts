import { t_main } from './main';

// In a *.test.ts file -> classified as a test by is_test_file, so calling t_main
// yields a test_of edge (cross-file).
export function test_t_main(): number {
    return t_main();
}
