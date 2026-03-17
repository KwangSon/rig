"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";

interface User {
  id: string;
  name: string;
  email: string;
}

export default function NewProjectPage() {
  const router = useRouter();
  const [user, setUser] = useState<User | null>(null);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    const token = localStorage.getItem("token");
    if (!token) {
      router.push("/auth/login");
      return;
    }
    fetchUser(token);
  }, [router]);

  const fetchUser = async (token: string) => {
    try {
      const res = await fetch("http://localhost:3000/api/v1/users/me", {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        setUser(await res.json());
      } else {
        router.push("/auth/login");
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name) return;

    setLoading(true);
    setError("");
    const token = localStorage.getItem("token");

    try {
      const res = await fetch("http://localhost:3000/api/v1/create_project", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ name }),
      });

      if (res.ok) {
        // Redirect to the newly created project page
        router.push(`/projects/${name}`);
      } else {
        const data = await res.json();
        setError(data.message || "Failed to create project");
      }
    } catch (err) {
      setError("Network error occurred while creating project");
    } finally {
      setLoading(false);
    }
  };

  if (!user) {
    return (
      <div className="flex min-h-[calc(100vh-56px)] justify-center bg-gray-50 py-20">
        <div className="h-10 w-10 animate-spin rounded-full border-b-2 border-indigo-600"></div>
      </div>
    );
  }

  return (
    <div className="flex min-h-[calc(100vh-56px)] justify-center bg-gray-50 px-4 py-10 sm:px-6 lg:px-8">
      <div className="w-full max-w-3xl">
        <div className="mb-8 border-b border-gray-200 pb-5">
          <h2 className="text-3xl leading-7 font-bold text-gray-900 sm:truncate sm:text-4xl sm:tracking-tight">
            Create a New Project
          </h2>
          <p className="mt-2 max-w-2xl text-sm text-gray-500">
            A project contains all asset files, including their revision
            history.
          </p>
        </div>

        <div className="bg-white px-6 py-8 shadow-sm ring-1 ring-gray-900/5 sm:rounded-xl md:col-span-2">
          <form className="space-y-8" onSubmit={handleSubmit}>
            {/* Owner / Name Row */}
            <div className="flex flex-col items-end gap-4 sm:flex-row sm:gap-6">
              <div className="w-full sm:w-1/3">
                <label className="block text-sm leading-6 font-semibold text-gray-900">
                  Owner
                </label>
                <div className="mt-2 flex items-center rounded-md border border-gray-300 bg-gray-50 px-3 py-2 shadow-sm">
                  <div className="mr-2 flex h-5 w-5 items-center justify-center rounded bg-indigo-100 text-xs font-bold text-indigo-700">
                    {user.name.charAt(0).toUpperCase()}
                  </div>
                  <span className="font-medium text-gray-700">{user.name}</span>
                </div>
              </div>

              <div className="hidden pb-2 text-2xl font-bold text-gray-500 sm:flex">
                /
              </div>

              <div className="w-full sm:flex-1">
                <label
                  htmlFor="project-name"
                  className="block text-sm leading-6 font-semibold text-gray-900"
                >
                  Project Name
                </label>
                <div className="mt-2">
                  <input
                    id="project-name"
                    name="project-name"
                    type="text"
                    required
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    className="block w-full rounded-md border-0 px-3 py-2 text-gray-900 shadow-sm ring-1 ring-gray-300 ring-inset placeholder:text-gray-400 focus:ring-2 focus:ring-indigo-600 focus:ring-inset sm:text-sm sm:leading-6"
                    placeholder="e.g. awesome-game-assets"
                  />
                </div>
              </div>
            </div>

            <div className="text-sm text-gray-500">
              Great project names are short and memorable. Need inspiration? How
              about{" "}
              <span className="font-medium text-gray-700">laughing-engine</span>
              ?
            </div>

            <div className="border-t border-gray-200 pt-8">
              <label
                htmlFor="description"
                className="block text-sm leading-6 font-semibold text-gray-900"
              >
                Description{" "}
                <span className="font-normal text-gray-400">(Optional)</span>
              </label>
              <div className="mt-2">
                <textarea
                  id="description"
                  name="description"
                  rows={3}
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  className="block w-full rounded-md border-0 px-3 py-2 text-gray-900 shadow-sm ring-1 ring-gray-300 ring-inset placeholder:text-gray-400 focus:ring-2 focus:ring-indigo-600 focus:ring-inset sm:text-sm sm:leading-6"
                  placeholder="Short description of your project"
                />
              </div>
            </div>

            {error && (
              <div className="rounded-md bg-red-50 p-4">
                <div className="flex">
                  <div className="flex-shrink-0">
                    <svg
                      className="h-5 w-5 text-red-400"
                      viewBox="0 0 20 20"
                      fill="currentColor"
                    >
                      <path
                        fillRule="evenodd"
                        d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z"
                        clipRule="evenodd"
                      />
                    </svg>
                  </div>
                  <div className="ml-3">
                    <h3 className="text-sm font-medium text-red-800">
                      {error}
                    </h3>
                  </div>
                </div>
              </div>
            )}

            <div className="mt-6 flex items-center justify-start gap-4 border-t border-gray-200 pt-6">
              <button
                type="submit"
                disabled={loading}
                className={`rounded-md px-6 py-2.5 text-sm font-semibold text-white shadow-sm transition-all ${loading ? "cursor-not-allowed bg-indigo-400" : "bg-indigo-600 hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"}`}
              >
                {loading ? "Creating..." : "Create Project"}
              </button>
              <Link
                href="/"
                className="text-sm leading-6 font-semibold text-gray-900 hover:text-gray-600"
              >
                Cancel
              </Link>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
