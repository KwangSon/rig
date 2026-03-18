"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";

const API_BASE = "http://localhost:3000/api/v1";

interface User {
  id: string;
  name: string;
  email: string;
}

interface User {
  id: string;
  name: string;
  email: string;
}

export default function UserSettingsPage() {
  const router = useRouter();
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState("");

  useEffect(() => {
    const t = localStorage.getItem("token");
    if (!t) {
      router.push("/auth/login");
      return;
    }
    setToken(t);
    fetchData(t);
  }, [router]);

  const fetchData = async (authToken: string) => {
    try {
      const userRes = await fetch(`${API_BASE}/users/me`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });

      if (userRes.ok) {
        setUser(await userRes.json());
      } else {
        router.push("/auth/login");
        return;
      }
    } catch (e) {
      console.error(e);
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
    <div className="min-h-[calc(100vh-56px)] bg-gray-50">
      <div className="mx-auto max-w-4xl px-4 py-10 sm:px-6 lg:px-8">
        <div className="mb-8 border-b border-gray-200 pb-5">
          <h1 className="text-3xl font-bold text-gray-900">User Settings</h1>
          <p className="mt-2 text-sm text-gray-500">
            Manage your account settings.
          </p>
        </div>

        <div className="space-y-10">
          {/* Profile Section */}
          <section>
            <h2 className="mb-4 text-xl font-semibold text-gray-900">
              Profile
            </h2>
            <div className="bg-white p-6 shadow ring-1 ring-gray-900/5 sm:rounded-lg">
              <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
                <div>
                  <label className="block text-sm font-medium text-gray-700">
                    Name
                  </label>
                  <p className="mt-1 text-sm text-gray-900">{user.name}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">
                    Email
                  </label>
                  <p className="mt-1 text-sm text-gray-900">{user.email}</p>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
