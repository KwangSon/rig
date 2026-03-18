"use client";

import Link from "next/link";

export default function Home() {
  return (
    <div className="bg-white">
      {/* Hero Section */}
      <div className="relative isolate bg-gradient-to-b from-indigo-50/50 to-white px-6 pt-14 lg:px-8">
        <div className="mx-auto max-w-4xl py-24 sm:py-32 lg:py-40">
          <div className="text-center">
            <div className="mb-8 inline-flex animate-bounce items-center gap-2 rounded-full bg-indigo-100 px-3 py-1 text-sm font-bold text-indigo-700">
              <span className="flex h-2 w-2 rounded-full bg-indigo-600"></span>
              Version 1.0 is here
            </div>
            <h1 className="mb-8 text-5xl font-extrabold tracking-tight text-gray-900 sm:text-7xl">
              Digital Asset Management{" "}
              <span className="text-indigo-600">Reimagined</span>
            </h1>
            <p className="mx-auto max-w-2xl text-lg leading-relaxed font-medium text-gray-600 sm:text-xl">
              Rig brings the power of Git to binary assets. Professional
              versioning, exclusive locking, and blazing-fast sync for creative
              teams working with large files.
            </p>
            <div className="mt-12 flex items-center justify-center gap-x-6">
              <Link
                href="/explore"
                className="rounded-lg bg-indigo-600 px-8 py-4 text-lg font-bold text-white shadow-lg transition-all hover:bg-indigo-700 hover:shadow-indigo-200/50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 active:scale-95"
              >
                Get Started
              </Link>
              <Link
                href="/help"
                className="text-lg font-bold text-gray-900 transition-colors hover:text-indigo-600"
              >
                Learn more <span aria-hidden="true">→</span>
              </Link>
            </div>
          </div>
        </div>
      </div>

      {/* Features Section */}
      <div className="bg-white py-24 sm:py-32">
        <div className="mx-auto max-w-7xl px-6 lg:px-8">
          <div className="mx-auto max-w-2xl lg:text-center">
            <h2 className="text-base font-bold tracking-widest text-indigo-600 uppercase">
              Core Capabilities
            </h2>
            <p className="mt-2 text-4xl font-extrabold tracking-tight text-gray-900 sm:text-5xl">
              Everything you need, nothing you don't.
            </p>
          </div>
          <div className="mx-auto mt-16 max-w-2xl sm:mt-20 lg:mt-24 lg:max-w-none">
            <dl className="grid max-w-xl grid-cols-1 gap-x-12 gap-y-16 lg:max-w-none lg:grid-cols-3">
              <div className="flex flex-col rounded-2xl border border-gray-100 bg-gray-50 p-8 transition-colors hover:border-indigo-200">
                <dt className="flex items-center gap-x-3 text-lg font-bold text-gray-900">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-indigo-600 text-white">
                    <svg
                      className="h-6 w-6"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
                      />
                    </svg>
                  </div>
                  Binary Locking
                </dt>
                <dd className="mt-4 flex flex-auto flex-col text-base leading-7 text-gray-600">
                  <p className="flex-auto">
                    Exclusive locks prevent merge conflicts on binary files.
                    Lock a 3D model, finish your edits, and push with total
                    confidence.
                  </p>
                </dd>
              </div>
              <div className="flex flex-col rounded-2xl border border-gray-100 bg-gray-50 p-8 transition-colors hover:border-indigo-200">
                <dt className="flex items-center gap-x-3 text-lg font-bold text-gray-900">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-indigo-600 text-white">
                    <svg
                      className="h-6 w-6"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                      />
                    </svg>
                  </div>
                  Lazy Syncing
                </dt>
                <dd className="mt-4 flex flex-auto flex-col text-base leading-7 text-gray-600">
                  <p className="flex-auto">
                    Don't waste space. Metadata is synced instantly, but heavy
                    binary payloads are downloaded only when you need them.
                  </p>
                </dd>
              </div>
              <div className="flex flex-col rounded-2xl border border-gray-100 bg-gray-50 p-8 transition-colors hover:border-indigo-200">
                <dt className="flex items-center gap-x-3 text-lg font-bold text-gray-900">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-indigo-600 text-white">
                    <svg
                      className="h-6 w-6"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"
                      />
                    </svg>
                  </div>
                  Git Modules
                </dt>
                <dd className="mt-4 flex flex-auto flex-col text-base leading-7 text-gray-600">
                  <p className="flex-auto">
                    Manage your source code and assets together. Rig integrates
                    seamlessly with standard Git repositories as modules.
                  </p>
                </dd>
              </div>
            </dl>
          </div>
        </div>
      </div>
    </div>
  );
}
